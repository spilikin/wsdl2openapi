import logging
from contextlib import contextmanager
from dataclasses import dataclass, field
from functools import singledispatchmethod
from typing import List, Tuple

import msgspec
from lxml.etree import QName
from zeep.xsd.elements import Any as XsdAny
from zeep.xsd.elements import (
    AnyAttribute,
    Choice,
    Element,
    Sequence,
)
from zeep.xsd.elements.indicators import OrderIndicator
from zeep.xsd.schema import Schema
from zeep.xsd.types import AnySimpleType, AnyType, ComplexType

from .common import Context, NamingStrategy
from .model import (
    Api,
    ReferenceObject,
    Type,
    TypeArray,
    TypeBoolean,
    TypeInteger,
    TypeNumber,
    TypeObject,
    TypeString,
    XmlExtension,
    schema_key,
    to_yaml,
)


@dataclass
class XmlSchemaVisitor:
    naming_strategy: NamingStrategy
    api: Api
    ctx: Context
    stack: List[TypeObject] = field(default_factory=list)
    components: List[Tuple[QName, Type]] = field(default_factory=list)
    global_types: dict = field(default_factory=dict)
    base_type_qnames: set = field(default_factory=set)

    # --- Stack management for current type being processed
    @contextmanager
    def push_type(self, type_declaration: TypeObject):
        self.stack.append(type_declaration)
        try:
            yield
        finally:
            self.stack.pop()

    def current_type(self) -> TypeObject:
        if not self.stack:
            raise ValueError("No type in the current scope.")
        return self.stack[-1]

    # ------------------------------------------------------------------------------

    def visit_schema(self, schema: Schema):
        self.components.clear()

        for type in schema.types:
            if (
                type.qname is not None
                and type.qname.namespace != "http://www.w3.org/2001/XMLSchema"
            ):
                self.global_types[str(type.qname)] = True

        for element in schema.elements:
            if isinstance(element, Element):
                self.visit_global_element(element.type, element)

        for global_type in schema.types:
            if str(global_type.qname) in self.global_types:
                self.visit_global_type(global_type)

        schemas = self.api.components.schemas
        for qname, type_declaration in self.components:
            id = self.naming_strategy.namespace_identifier(self.ctx, qname.namespace)
            type_name = self.naming_strategy.format_type_name(qname.localname)
            key = schema_key(id, type_name)
            existing = schemas.get(key)
            if existing is not None:
                if isinstance(existing, TypeObject) and isinstance(
                    type_declaration, TypeObject
                ):
                    # Same type defined in two XSDs of the same namespace
                    # (e.g., different version snapshots): merge in any
                    # properties the existing declaration is missing so we
                    # don't silently drop fields like inline-typed elements.
                    for prop_name, prop_type in type_declaration.properties.items():
                        if prop_name not in existing.properties:
                            existing.properties[prop_name] = prop_type
                else:
                    logging.debug(
                        "Type already exists in schema %s: %s, keeping existing type",
                        id,
                        type_name,
                    )
                continue
            schemas[key] = type_declaration

        for base_qname in self.base_type_qnames:
            id = self.naming_strategy.namespace_identifier(
                self.ctx, base_qname.namespace
            )
            type_name = self.naming_strategy.format_type_name(base_qname.localname)
            key = schema_key(id, type_name)
            if key in schemas:
                xml_ext = self.get_xml_ext_for_type(schemas[key])
                xml_ext.is_base = True

    @singledispatchmethod
    def visit_global_type(self, type: any):
        raise TypeError(f"Unsupported global type: {type}")

    @visit_global_type.register
    def _(self, complex_type: ComplexType):
        type_declaration = TypeObject()
        with self.push_type(type_declaration):
            self.visit_complex_type(complex_type)

        self.components.append((complex_type.qname, type_declaration))

    @visit_global_type.register
    def _(self, simple_type: AnySimpleType):
        type_declaration = infere_primitive_type(simple_type)
        self.components.append((simple_type.qname, type_declaration))

    @singledispatchmethod
    def visit_global_element(self, element_type: any, element: Element):
        raise TypeError(
            f"Unsupported global element: {element} of type {type(element_type)}"
        )

    @visit_global_element.register
    def _(self, element_type: AnySimpleType, element: Element):
        """Visit a global element of simple type."""
        if self.is_simple_type_global(element_type):
            type_declaration = ReferenceObject(
                self.naming_strategy.reference_to_type(self.ctx, element_type.qname),
                xml=create_xml_extension(element.qname),
            )
        else:
            type_declaration = infere_primitive_type(element_type)
            type_declaration.xml = create_xml_extension(element.qname)

        self.components.append((element.qname, type_declaration))

    @visit_global_element.register
    def _(self, element_type: ComplexType, element: Element):
        """Visit a global element of complex type."""
        # global types are referenced, inline complex types are defined inline
        if element_type.is_global:
            # ugly workaround: if type is global and has the same name as the element (grr)
            # add "Type" to the type name to avoid name clashes
            if element_type.qname == element.qname:
                del self.global_types[str(element_type.qname)]
                element_type.qname = QName(str(element_type.qname) + "ElementType")
                self.global_types[str(element_type.qname)] = True

            type_declaration = ReferenceObject(
                self.naming_strategy.reference_to_type(self.ctx, element_type.qname),
                xml=create_xml_extension(element.qname),
            )

            self.base_type_qnames.add(element_type.qname)
            self.components.append((element.qname, type_declaration))
        else:
            type_declaration = TypeObject(xml=create_xml_extension(element.qname))
            with self.push_type(type_declaration):
                self.visit_complex_type(element_type)
            self.components.append((element.qname, type_declaration))

    @visit_global_element.register
    def _(self, type: AnyType, element: Element):
        """Visit a global element of anyType."""
        type_declaration = TypeString(
            xml=create_xml_extension(element.qname),
            format="xml",
        )
        self.components.append((element.qname, type_declaration))

    @singledispatchmethod
    def build_type(self, source: any) -> Type:
        raise TypeError(f"Unsupported source type: {type(source)}")

    @build_type.register
    def _(self, simple_type: AnySimpleType) -> Type:
        if self.is_simple_type_global(simple_type):
            return ReferenceObject(
                self.naming_strategy.reference_to_type(self.ctx, simple_type.qname)
            )
        else:
            return infere_primitive_type(simple_type)

    @build_type.register
    def _(self, complex_type: ComplexType) -> Type:
        if not complex_type.is_global:
            type_declaration = TypeObject()
            with self.push_type(type_declaration):
                self.visit_complex_type(complex_type)
            return type_declaration
        else:
            return ReferenceObject(
                self.naming_strategy.reference_to_type(self.ctx, complex_type.qname)
            )

    @build_type.register
    def _(self, any_type: AnyType) -> Type:
        return TypeString(
            format="xml",
        )

    @singledispatchmethod
    def visit_particle(self, particle: any):
        raise TypeError(f"Unsupported particle type: {type(particle)}")

    @visit_particle.register
    def _(self, particle: OrderIndicator):
        self.visit_particle_group(particle)

    @visit_particle.register
    def _(self, element: Element):
        if element.qname == "_value_1":
            property_name = self.naming_strategy.char_data_property_name()
            property_type = self.build_type(element.type)
            if isinstance(property_type, ReferenceObject):
                property_type = TypeString()
            property_type.format = "chardata"
            self.set_property_declaration(property_name, property_type)
        else:
            logging.error(
                "Unsupported element in Element: %s",
                element.type,
            )

    def visit_extensions(self, complex_type: ComplexType, type_declaration: Type):
        if len(complex_type._extension_types) > 0:
            extensions = []
            for ext_type in complex_type._extension_types:
                if "_xsd_type" in dir(ext_type):
                    extensions.append(
                        self.naming_strategy.reference_to_type(
                            self.ctx, ext_type._xsd_type.qname
                        )
                    )
                    self.base_type_qnames.add(ext_type._xsd_type.qname)
            self.get_xml_ext_for_type(type_declaration).extends = extensions

    def visit_complex_type(self, complex_type: ComplexType):
        self.visit_extensions(complex_type, self.current_type())
        # collect properties from attributes
        for _, attr in complex_type.attributes:
            if isinstance(attr, AnyAttribute):
                property_declaration = TypeString(format="xml")
                property_name = self.naming_strategy.unknown_attribute_property_name()
                property_declaration.xml = XmlExtension(
                    name="anyAttribute",
                    attribute=True,
                )
                self.set_property_declaration(
                    property_name, property_declaration, optional=True
                )
                continue

            property_name = self.naming_strategy.format_property_name(
                attr.qname.localname
            )
            property_declaration = self.build_type(attr.type)
            property_declaration.xml = XmlExtension(
                name=attr.qname.localname,
                attribute=True,
            )
            if attr.qname.namespace:
                property_declaration.xml.namespace = attr.qname.namespace

            self.set_property_declaration(
                property_name, property_declaration, optional=not attr.required
            )

        if len(complex_type.elements_nested) == 0:
            # complex type has no elements defined, only attributes
            return
        elif len(complex_type.elements_nested) > 1:
            logging.error(
                "Complex type has multiple nested particles, only one is supported: %s, Elements: %s",
                complex_type.qname,
                [el for _, el in complex_type.elements_nested],
            )
            raise ValueError(
                "Complex type has multiple nested particles, only one is supported."
            )

        _, particle = complex_type.elements_nested[0]

        self.visit_particle(particle)

    def build_property_declaration(
        self, element: Element
    ) -> Tuple[str, Type, bool]:
        property_name = self.naming_strategy.format_property_name(
            str(element.qname.localname)
        )

        if element.is_global:
            # reference to a global element
            property_type = ReferenceObject(
                self.naming_strategy.reference_to_type(self.ctx, element.qname),
            )
        else:
            element_type = self.build_type(element.type)
            element_type.xml = create_xml_extension(element.qname)
            property_type = element_type

        if is_multi_occurs(element.max_occurs):
            property_type = TypeArray(items=property_type)

        optional = element.min_occurs == 0

        return property_name, property_type, optional

    def set_property_declaration(
        self,
        property_name: str,
        property_type: Type,
        optional: bool = False,
    ) -> None:
        type_declaration = self.current_type()
        if property_name in type_declaration.properties:
            # TODO: handle name clashes properly (especially cardinality differences)
            logging.debug(
                "Possible property name clash for %s in type %s, overwriting existing property: %s",
                property_name,
                to_yaml(property_type),
                to_yaml(type_declaration.properties[property_name]),
            )
            return
        type_declaration.properties[property_name] = property_type
        if not optional:
            if type_declaration.required is msgspec.UNSET:
                type_declaration.required = []
            type_declaration.required.append(property_name)

    def visit_particle_group(self, particle: OrderIndicator):
        for _, el in particle.elements_nested:
            if isinstance(el, Element):
                logging.debug(
                    "Visiting element: %s, maxOccurs: %s", el.qname, el.max_occurs
                )
                property_name, property_type, optional = (
                    self.build_property_declaration(el)
                )
                if is_multi_occurs(particle.max_occurs) and not is_multi_occurs(
                    el.max_occurs
                ):
                    property_type = TypeArray(items=property_type)
                if el.min_occurs == 0:
                    optional = True
                self.set_property_declaration(
                    property_name, property_type, optional=optional
                )
            elif isinstance(el, Sequence):
                self.visit_particle(el)
            elif isinstance(el, Choice):
                self.visit_particle(el)
            elif isinstance(el, XsdAny):
                property_name = self.naming_strategy.unknown_content_property_name()
                property_type = TypeString(
                    format="xml",
                )
                self.set_property_declaration(
                    property_name, property_type, optional=True
                )
            else:
                logging.error(
                    "Unsupported element type in: %s, Element: %s",
                    type(el),
                    el,
                )
                continue

    def is_simple_type_global(self, simple_type: AnySimpleType) -> bool:
        if not simple_type.is_global:
            return False

        return str(simple_type.qname) in self.global_types

    def get_xml_ext_for_type(self, type_declaration: Type) -> XmlExtension:
        if type_declaration.xml is None or type_declaration.xml == msgspec.UNSET:
            type_declaration.xml = XmlExtension()

        return type_declaration.xml


def is_multi_occurs(max_occurs) -> bool:
    return (isinstance(max_occurs, int) and max_occurs > 1) or max_occurs == "unbounded"


def create_xml_extension(qname: QName) -> XmlExtension:
    return XmlExtension(
        name=qname.localname,
        namespace=qname.namespace,
    )


def infere_primitive_type(simple_type: AnySimpleType) -> Type:
    type_name = str(simple_type._default_qname)
    if type_name == "{http://www.w3.org/2001/XMLSchema}string":
        return TypeString()
    elif type_name == "{http://www.w3.org/2001/XMLSchema}boolean":
        return TypeBoolean()
    elif type_name in (
        "{http://www.w3.org/2001/XMLSchema}decimal",
        "{http://www.w3.org/2001/XMLSchema}float",
        "{http://www.w3.org/2001/XMLSchema}double",
    ):
        return TypeNumber()
    elif type_name in (
        "{http://www.w3.org/2001/XMLSchema}integer",
        "{http://www.w3.org/2001/XMLSchema}int",
        "{http://www.w3.org/2001/XMLSchema}long",
        "{http://www.w3.org/2001/XMLSchema}short",
        "{http://www.w3.org/2001/XMLSchema}byte",
        "{http://www.w3.org/2001/XMLSchema}unsignedLong",
        "{http://www.w3.org/2001/XMLSchema}unsignedInt",
        "{http://www.w3.org/2001/XMLSchema}unsignedShort",
        "{http://www.w3.org/2001/XMLSchema}unsignedByte",
    ):
        return TypeInteger()
    elif type_name == "{http://www.w3.org/2001/XMLSchema}nonNegativeInteger":
        return TypeInteger(minimum=0)
    elif type_name == "{http://www.w3.org/2001/XMLSchema}nonPositiveInteger":
        return TypeInteger(maximum=0)
    elif type_name == "{http://www.w3.org/2001/XMLSchema}positiveInteger":
        return TypeInteger(minimum=1)
    elif type_name == "{http://www.w3.org/2001/XMLSchema}negativeInteger":
        return TypeInteger(maximum=-1)
    elif type_name == "{http://www.w3.org/2001/XMLSchema}dateTime":
        return TypeString(format="date-time")
    elif type_name == "{http://www.w3.org/2001/XMLSchema}date":
        return TypeString(format="date")
    elif type_name == "{http://www.w3.org/2001/XMLSchema}time":
        return TypeString(format="time")
    elif type_name == "{http://www.w3.org/2001/XMLSchema}base64Binary":
        return TypeString(format="byte")
    elif type_name == "{http://www.w3.org/2001/XMLSchema}hexBinary":
        return TypeString(format="hex")
    elif type_name == "{http://www.w3.org/2001/XMLSchema}anyURI":
        return TypeString(format="uri")
    else:
        return TypeString()  # Default to string if type is unknown
