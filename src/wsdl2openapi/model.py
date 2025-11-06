import json
from collections import OrderedDict
from enum import Enum
from typing import Dict, List, Optional

import msgspec


class BindingType(Enum):
    SOAP11 = "soap11"
    SOAP12 = "soap12"
    HTTP_GET = "http-get"
    HTTP_POST = "http-post"
    OTHER = "other"


class WebServiceMessage(msgspec.Struct):
    soapBody: str = msgspec.field()
    soapHeaders: Optional[List[str]] = msgspec.field(default=msgspec.UNSET)


class OperationDefinition(msgspec.Struct):
    name: str
    soapAction: Optional[str] = msgspec.field(default=msgspec.UNSET)
    style: Optional[str] = msgspec.field(default=msgspec.UNSET)
    input: WebServiceMessage = None
    output: WebServiceMessage = None
    faults: Optional[Dict[str, WebServiceMessage]] = msgspec.field(
        default=msgspec.UNSET
    )
    inputHeaders: Optional[Dict[str, WebServiceMessage]] = msgspec.field(
        default=msgspec.UNSET
    )
    outputHeaders: Optional[Dict[str, WebServiceMessage]] = msgspec.field(
        default=msgspec.UNSET
    )


class WebServicePort(msgspec.Struct):
    name: str
    bindingType: BindingType
    address: Optional[str] = msgspec.field(default=msgspec.UNSET)
    operations: List[OperationDefinition] = msgspec.field(default_factory=list)


class WebService(msgspec.Struct):
    name: str
    targetNamespace: str
    targetPackage: str
    ports: List[WebServicePort] = msgspec.field(default_factory=list)


class Components(msgspec.Struct):
    schemas: OrderedDict[str, "OrderedDict[str, Type | ReferenceObject]"] = (
        msgspec.field(default_factory=OrderedDict)
    )

    def schema_dict(self, path: List[str]) -> OrderedDict:
        """Return the dictionary for a given path in the components, creates the necessary structure if it doesn't exist."""
        current = self.schemas
        for part in path:
            if part not in current:
                current[part] = OrderedDict()
            current = current[part]
        return current


class License:
    name: str
    identifier: Optional[str] = msgspec.field(default=msgspec.UNSET)
    url: Optional[str] = msgspec.field(default=msgspec.UNSET)


class Contact:
    name: Optional[str] = msgspec.field(default=msgspec.UNSET)
    email: Optional[str] = msgspec.field(default=msgspec.UNSET)
    url: Optional[str] = msgspec.field(default=msgspec.UNSET)


class Info(msgspec.Struct):
    title: str
    version: str
    description: Optional[str] = msgspec.field(default=msgspec.UNSET)
    termsOfService: Optional[str] = msgspec.field(default=msgspec.UNSET)
    license: Optional[License] = msgspec.field(default=msgspec.UNSET)
    contact: Optional[Contact] = msgspec.field(default=msgspec.UNSET)


class Operation(msgspec.Struct):
    summary: Optional[str] = msgspec.field(default=msgspec.UNSET)
    description: Optional[str] = msgspec.field(default=msgspec.UNSET)
    operationId: Optional[str] = msgspec.field(default=msgspec.UNSET)
    tags: Optional[List[str]] = msgspec.field(default=msgspec.UNSET)


class PathItem(msgspec.Struct):
    post: Optional[Operation] = msgspec.field(default=msgspec.UNSET)
    get: Optional[Operation] = msgspec.field(default=msgspec.UNSET)
    put: Optional[Operation] = msgspec.field(default=msgspec.UNSET)
    delete: Optional[Operation] = msgspec.field(default=msgspec.UNSET)
    options: Optional[Operation] = msgspec.field(default=msgspec.UNSET)
    head: Optional[Operation] = msgspec.field(default=msgspec.UNSET)
    patch: Optional[Operation] = msgspec.field(default=msgspec.UNSET)
    trace: Optional[Operation] = msgspec.field(default=msgspec.UNSET)


class Api(msgspec.Struct):
    openapi: str = "3.1.0"
    wsdl_version: str = msgspec.field(default="1.0.0", name="x-wsdl-version")
    info: Info = msgspec.field(
        default_factory=lambda: Info(title="API", version="1.0.0")
    )
    wsdl_services: Optional[List[WebService]] = msgspec.field(
        default=msgspec.UNSET, name="x-wsdl-services"
    )
    paths: Optional[OrderedDict[str, PathItem]] = msgspec.field(default=msgspec.UNSET)
    components: Components = msgspec.field(default_factory=Components)


class XmlExtension(msgspec.Struct):
    name: Optional[str] = msgspec.field(default=msgspec.UNSET)
    namespace: Optional[str] = msgspec.field(default=msgspec.UNSET)
    prefix: Optional[str] = msgspec.field(default=msgspec.UNSET)
    attribute: Optional[bool] = msgspec.field(default=msgspec.UNSET)
    wrapped: Optional[bool] = msgspec.field(default=msgspec.UNSET)
    extends: Optional[List[str]] = msgspec.field(
        name="x-extends", default=msgspec.UNSET
    )
    is_base: Optional[bool] = msgspec.field(default=msgspec.UNSET, name="x-is-base")


class ReferenceObject(msgspec.Struct):
    ref: str = msgspec.field(name="$ref")
    xml: Optional[XmlExtension] = msgspec.field(default=msgspec.UNSET)
    summary: Optional[str] = msgspec.field(default=msgspec.UNSET)
    description: Optional[str] = msgspec.field(default=msgspec.UNSET)
    nullable: Optional[bool] = msgspec.field(default=msgspec.UNSET)

    def __str__(self):
        return self.ref


class Type(msgspec.Struct, kw_only=True, tag=True, tag_field="type"):
    xml: Optional[XmlExtension] = msgspec.field(default=msgspec.UNSET)
    description: Optional[str] = msgspec.field(default=msgspec.UNSET)
    nullable: Optional[bool] = msgspec.field(default=msgspec.UNSET)
    enum: Optional[List[str]] = msgspec.field(default=msgspec.UNSET)


class TypeObject(Type, tag="object"):
    properties: OrderedDict[str, Type | ReferenceObject] = msgspec.field(
        default_factory=OrderedDict
    )


class TypeString(Type, tag="string"):
    pattern: Optional[str] = msgspec.field(default=msgspec.UNSET)
    minLength: Optional[int] = msgspec.field(default=msgspec.UNSET)
    maxLength: Optional[int] = msgspec.field(default=msgspec.UNSET)
    format: Optional[str] = msgspec.field(default=msgspec.UNSET)
    contentMediaType: Optional[str] = msgspec.field(default=msgspec.UNSET)
    contentEncoding: Optional[str] = msgspec.field(default=msgspec.UNSET)


class TypeNumber(Type, tag="number"):
    multipleOf: Optional[float] = msgspec.field(default=msgspec.UNSET)
    maximum: Optional[float] = msgspec.field(default=msgspec.UNSET)
    exclusiveMaximum: Optional[float] = msgspec.field(default=msgspec.UNSET)
    minimum: Optional[float] = msgspec.field(default=msgspec.UNSET)
    exclusiveMinimum: Optional[float] = msgspec.field(default=msgspec.UNSET)


class TypeBoolean(Type, tag="boolean"):
    pass


class TypeInteger(TypeNumber, tag="integer"):
    pass


class TypeNull(Type, tag="null"):
    pass


class TypeArray(Type, tag="array"):
    items: Type | ReferenceObject


def to_yaml(obj: any) -> str:
    """Convert an object to a YAML string."""
    return msgspec.yaml.encode(obj).decode("utf-8")


def to_json(obj: any, indent: Optional[int] = None) -> str:
    """Convert an object to a JSON string."""
    if indent is None:
        return msgspec.json.encode(obj).decode("utf-8")
    else:
        raw = msgspec.json.encode(obj).decode("utf-8")
        return json.dumps(msgspec.json.decode(raw), indent=indent)
