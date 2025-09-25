import logging
from collections import OrderedDict
from contextlib import contextmanager
from dataclasses import dataclass, field
from functools import singledispatchmethod
from typing import List, Tuple

from lxml.etree import QName
from zeep.xsd.elements import Any as XsdAny
from zeep.xsd.elements import Choice, Element, Sequence
from zeep.xsd.schema import Schema
from zeep.xsd.types import AnySimpleType, AnyType, ComplexType

from .common import Context, NamingStrategy
from .model import (
    Api,
    Components,
    ReferenceObject,
    Type,
    TypeArray,
    TypeBoolean,
    TypeInteger,
    TypeNumber,
    TypeObject,
    TypeString,
    XmlExtension,
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
        for element in schema.elements:
            if isinstance(element, Element):
                self.visit_global_element(element.type, element)
        for type in schema.types:
            if (
                type.qname is not None
                and type.qname.namespace != "http://www.w3.org/2001/XMLSchema"
            ):
                self.visit_global_type(type)
                self.global_types[type.qname] = type

        for qname, type_declaration in self.components:
            id = self.naming_strategy.namespace_identifier(self.ctx, qname.namespace)
            schema_dict = create_schema_dict(self.api.components, [id])
            type_name = self.naming_strategy.format_type_name(qname.localname)
            if type_name in schema_dict:
                logging.debug(
                    "Type already exists in schema %s: %s, overwriting existing type",
                    id,
                    type_name,
                )

                continue
            schema_dict[self.naming_strategy.format_type_name(qname.localname)] = (
                type_declaration
            )

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
        if (
            element_type.is_global
            and element_type.qname.namespace == "http://www.w3.org/2001/XMLSchema"
        ):
            type_declaration = infere_primitive_type(element_type)
            type_declaration.xml = create_xml_extension(element.qname)
        elif element_type.is_global:
            type_declaration = infere_primitive_type(element_type)
            type_declaration.xml = create_xml_extension(element.qname)
        else:
            type_declaration = ReferenceObject(
                self.naming_strategy.reference_to_type(self.ctx, element_type.qname),
                xml=create_xml_extension(element.qname),
            )
        self.components.append((element.qname, type_declaration))

    @visit_global_element.register
    def _(self, element_type: ComplexType, element: Element):
        """Visit a global element of complex type."""
        # global types are referenced, inline complex types are defined inline
        if element_type.is_global:
            # ugly workaround: if type is global and has the same name as the element (grr)
            # add "Type" to the type name to avoid name clashes
            if element_type.qname == element.qname:
                element_type.qname = QName(str(element_type.qname) + "ElementType")
            type_declaration = ReferenceObject(
                self.naming_strategy.reference_to_type(self.ctx, element_type.qname),
                xml=create_xml_extension(element.qname),
            )
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
            description="This field can contain any XML content (xsd:anyType).",
        )
        self.components.append((element.qname, type_declaration))

    @singledispatchmethod
    def build_type(self, source: any) -> Type:
        raise TypeError(f"Unsupported source type: {type(source)}")

    @build_type.register
    def _(self, simple_type: AnySimpleType) -> Type:
        if (
            simple_type.qname.namespace == "http://www.w3.org/2001/XMLSchema"
            or not self.is_simple_type_global(simple_type)
        ):
            return infere_primitive_type(simple_type)
        else:
            return ReferenceObject(
                self.naming_strategy.reference_to_type(self.ctx, simple_type.qname)
            )

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
            description="This field can contain any XML content (xsd:anyType).",
        )

    @singledispatchmethod
    def visit_particle(self, particle: any):
        raise TypeError(f"Unsupported particle type: {type(particle)}")

    @visit_particle.register
    def _(self, sequence: Sequence):
        self.visit_particle_group(sequence.elements_nested)

    @visit_particle.register
    def _(self, choice: Choice):
        self.visit_particle_group(choice.elements_nested)

    @visit_particle.register
    def _(self, element: Element):
        if element.qname == "_value_1":
            property_name = self.naming_strategy.char_data_property_name()
            self.set_property_declaration(property_name, TypeString())
        else:
            logging.error(
                "Unsupported element in Element: %s",
                element.type,
            )

    def visit_complex_type(self, complex_type: ComplexType):
        # collect properties from attributes
        for _, attr in complex_type.attributes:
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

            self.set_property_declaration(property_name, property_declaration)

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

    def build_property_declaration(self, element: Element) -> Tuple[str, Type]:
        property_name = self.naming_strategy.format_property_name(
            str(element.qname.localname)
        )

        element_type = self.build_type(element.type)
        element_type.xml = create_xml_extension(element.qname)

        if is_multi_occurs(element.max_occurs):
            property_type = TypeArray(items=element_type)
        else:
            property_type = element_type

        return property_name, property_type

    def set_property_declaration(self, property_name: str, property_type: Type) -> None:
        type_declaration = self.current_type()
        if property_name in type_declaration.properties:
            logging.warning(
                "Possible property name clash for %s in type %s, overwriting existing property: %s",
                property_name,
                type_declaration.xml,
                to_yaml(type_declaration.properties[property_name]),
            )
            return
        type_declaration.properties[property_name] = property_type

    def visit_particle_group(self, nested_elements: list):
        for _, el in nested_elements:
            if isinstance(el, Element):
                logging.debug(
                    "Visiting element: %s, maxOccurs: %s", el.qname, el.max_occurs
                )
                property_name, property_type = self.build_property_declaration(el)
                self.set_property_declaration(property_name, property_type)
            elif isinstance(el, Sequence):
                self.visit_particle(el)
            elif isinstance(el, Choice):
                self.visit_particle(el)
            elif isinstance(el, XsdAny):
                property_name = self.naming_strategy.unknown_content_property_name()
                property_type = TypeString(
                    format="xml",
                    description="This field can contain any XML content (xsd:any).",
                )
                self.set_property_declaration(property_name, property_type)
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

        return simple_type.qname in self.global_types


def create_schema_dict(components: Components, path: List[str]) -> OrderedDict:
    """Return the dictionary for a given path in the components, creates the necessary structure if it doesn't exist."""
    current = components.schemas
    for part in path:
        if part not in current:
            current[part] = OrderedDict()
        current = current[part]
    return current


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
