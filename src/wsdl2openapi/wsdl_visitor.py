import logging
from dataclasses import dataclass
from functools import singledispatchmethod
from typing import OrderedDict

import msgspec
from zeep.wsdl.bindings.http import HttpGetBinding, HttpPostBinding
from zeep.wsdl.bindings.soap import (
    DocumentMessage as SoapDocumentMessage,
)
from zeep.wsdl.bindings.soap import (
    Soap11Binding,
    Soap12Binding,
    SoapOperation,
)
from zeep.wsdl.definitions import Binding, Definition, Port, Service
from zeep.xsd.elements.element import Element
from zeep.xsd.elements.indicators import All

from wsdl2openapi.common import Context, NamingStrategy
from wsdl2openapi.model import Api

from .model import (
    BindingType,
    OperationDefinition,
    WebService,
    WebServiceMessage,
    WebServicePort,
)


@dataclass
class WsdlVisitor:
    naming_strategy: NamingStrategy
    api: Api
    ctx: Context
    operations: OrderedDict[str, OperationDefinition] = msgspec.field(
        default_factory=OrderedDict
    )

    def visit_definition(self, definition: Definition):
        if self.api.wsdl_services is msgspec.UNSET:
            self.api.wsdl_services = []
        for service in definition.services.values():
            self.visit_service(definition, service)

    def visit_service(self, definition: Definition, service: Service):
        namespace_id = self.naming_strategy.namespace_identifier(
            self.ctx, definition.target_namespace
        )
        wsdl_service = WebService(
            name=service.name,
            targetNamespace=definition.target_namespace,
            targetPackage=namespace_id,
        )
        self.api.wsdl_services.append(wsdl_service)
        for port in service.ports.values():
            self.visit_port(definition, port, wsdl_service)

    def visit_port(self, definition: Definition, port: Port, wsdl_service: WebService):
        wsdl_port = WebServicePort(
            name=port.name,
            bindingType=infere_binding_type(port.binding),
            address=port.binding_options.get("address"),
        )
        if (
            wsdl_port.bindingType == BindingType.OTHER
            or wsdl_port.bindingType == BindingType.HTTP_GET
            or wsdl_port.bindingType == BindingType.HTTP_POST
        ):
            logging.warning("Skipping non-SOAP binding: %s", wsdl_port.bindingType)
            return

        binding = definition.bindings.get(port.binding.name)
        for operation_name in binding.all():
            operation = binding.get(operation_name)
            self.visit_operation(operation, wsdl_port)

        wsdl_service.ports.append(wsdl_port)

    @singledispatchmethod
    def visit_operation(self, operation: any, wsdl_port: WebServicePort):
        raise NotImplementedError(f"Operation type {type(operation)} not supported")

    @visit_operation.register
    def _(self, operation: SoapOperation, wsdl_port: WebServicePort):
        op_def = OperationDefinition(
            name=operation.name,
            style=operation.style,
        )
        if op_def.style != "document":
            logging.warning(
                "Skipping non-document style SOAP operation: %s", op_def.style
            )
            return

        if operation.soapaction:
            op_def.soapAction = operation.soapaction

        op_def.input = self.build_document_message(operation.input)
        op_def.output = self.build_document_message(operation.output)
        op_def.faults = {}
        for fault_name, fault in operation.faults.items():
            op_def.faults[fault_name] = self.build_document_message(fault)

        if len(op_def.faults) == 0:
            op_def.faults = msgspec.UNSET

        wsdl_port.operations.append(op_def)

    def build_document_message(
        self, document_message: SoapDocumentMessage
    ) -> WebServiceMessage:
        part = next(iter(document_message.abstract.parts.values()))
        if part.element is None:
            raise NotImplementedError("Only element-based messages are supported")

        soap_headers = msgspec.UNSET
        if len(document_message.header.type.elements_nested) > 0:
            _, all_headers = document_message.header.type.elements_nested[0]
            if not isinstance(all_headers, All):
                raise NotImplementedError(
                    "Only 'All' indicator for SOAP headers is supported, got %s"
                    % type(all_headers)
                )

            soap_headers = []
            for header in all_headers:
                if not isinstance(header, Element):
                    raise NotImplementedError(
                        "Only Element headers are supported, got %s" % type(header)
                    )
                soap_headers.append(
                    self.naming_strategy.reference_to_type(self.ctx, header.qname)
                )

        return WebServiceMessage(
            soapBody=self.naming_strategy.reference_to_type(
                self.ctx, part.element.qname
            ),
            soapHeaders=soap_headers,
        )


def infere_binding_type(binding: Binding) -> BindingType:
    if isinstance(binding, Soap11Binding):
        return BindingType.SOAP11
    elif isinstance(binding, Soap12Binding):
        return BindingType.SOAP12
    elif isinstance(binding, HttpGetBinding):
        return BindingType.HTTP_GET
    elif isinstance(binding, HttpPostBinding):
        return BindingType.HTTP_POST
    else:
        return BindingType.OTHER
