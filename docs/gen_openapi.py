"""Write the OpenAPI document of the server's HTTP APIs for the docs site: `just docs-reference`
runs it, and the API reference page (reference/http-api.md) shows it with Scalar.

It reads the API from the protos in proto/, the source of the server's routes and of the
generated clients, through a descriptor set that `protoc` writes with their comments (the path
of the file is the one argument), and writes docs/reference/openapi.json: a tag per service, with
its comment, grouped by API as the REST APIs page groups them; an operation per RPC, with its method
and path from `google.api.http`, its path and query parameters, its request body and its
response; and a schema per message and enum, with the comments of their fields and values.

The JSON names and the encodings are those of the generated clients, from the constants of
crates/uc-codegen/src/main.rs: the packages whose JSON names are lowerCamelCase, the RPCs whose
body is a form, and those whose body keeps the path values.
"""

from __future__ import annotations

import json
import re
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

from google.api import annotations_pb2, field_behavior_pb2
from google.protobuf import descriptor_pb2

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "reference" / "openapi.json"

FDP = descriptor_pb2.FieldDescriptorProto
INTEGERS = {
    FDP.TYPE_INT32: "int32",
    FDP.TYPE_SINT32: "int32",
    FDP.TYPE_SFIXED32: "int32",
    FDP.TYPE_UINT32: "int64",
    FDP.TYPE_FIXED32: "int64",
    FDP.TYPE_INT64: "int64",
    FDP.TYPE_SINT64: "int64",
    FDP.TYPE_SFIXED64: "int64",
    FDP.TYPE_UINT64: "int64",
    FDP.TYPE_FIXED64: "int64",
}
# The schemas of the well-known types, as their JSON mapping.
WELL_KNOWN = {
    ".google.protobuf.Struct": {"type": "object"},
    ".google.protobuf.Value": {},
    ".google.protobuf.ListValue": {"type": "array", "items": {}},
    ".google.protobuf.Empty": {"type": "object"},
    ".google.protobuf.Timestamp": {"type": "string", "format": "date-time"},
}

# The APIs of the server (docs/clients/rest-apis.md), as tag groups: the services in each. A
# service not named here is in the Unity Catalog REST API.
GROUPS = {
    "Delta REST API": ["DeltaApiService"],
    "Iceberg REST catalog": ["IcebergService"],
    "Delta Sharing": ["SharingService"],
    "Control plane": ["AuthService", "ScimService"],
    "Health": ["HealthService"],
}
DEFAULT_GROUP = "Unity Catalog REST API"
# The field options of trestle (proto/olai/trestle/v1/annotations.proto): a field of a response
# that comes from a response header, and a field that is the HTTP body as raw bytes.
RESPONSE_HEADER = 51001
RAW_BODY = 51003
# Words of service names that are acronyms or brand names.
WORDS = {"Api": "API", "Scim": "SCIM", "Rest": "REST"}


def rust_list(name: str) -> list[str]:
    """The strings of `const <name>: &[&str] = &[...]` in uc-codegen."""
    source = (ROOT / "crates" / "uc-codegen" / "src" / "main.rs").read_text()
    match = re.search(rf"const {name}: &\[&str\] = &\[(.*?)\];", source, re.DOTALL)
    if match is None:
        sys.exit(f"crates/uc-codegen/src/main.rs has no `const {name}`")
    return re.findall(r'"([^"]+)"', match.group(1))


CAMEL_CASE_PACKAGES = set(rust_list("CAMEL_CASE_PACKAGES"))
FORM_METHODS = set(rust_list("FORM_METHODS"))
PATH_FIELDS_IN_BODY_METHODS = set(rust_list("PATH_FIELDS_IN_BODY_METHODS"))


def lower_camel(name: str) -> str:
    head, *rest = name.split("_")
    return head + "".join(p[:1].upper() + p[1:] for p in rest)


def summary(text: str) -> str:
    """The first sentence of a comment, up to a colon: the title of an operation."""
    text = " ".join(text.split()).replace("`", "")
    return text.split(". ")[0].split(": ")[0].rstrip(".")


def tag_name(service: str) -> str:
    """`TemporaryCredentialsService` -> "Temporary credentials"."""
    words = re.findall(r"[A-Z][a-z0-9]*", service.removesuffix("Service"))
    words = [WORDS.get(w, w) for w in words]
    return " ".join([words[0], *(w if w.isupper() else w.lower() for w in words[1:])])


@dataclass
class Message:
    full_name: str
    name: str
    package: str
    proto: descriptor_pb2.DescriptorProto
    comment: str
    field_comments: dict[int, str]


@dataclass
class Enum:
    full_name: str
    name: str
    package: str
    proto: descriptor_pb2.EnumDescriptorProto
    comment: str
    value_comments: dict[int, str]


def varint(data: bytes, at: int) -> tuple[int, int]:
    """The varint at `at` of `data`, and the offset after it."""
    value = shift = 0
    while True:
        byte = data[at]
        at += 1
        value |= (byte & 0x7F) << shift
        shift += 7
        if byte < 0x80:
            return value, at


def trestle_options(f: descriptor_pb2.FieldDescriptorProto) -> dict[int, int | bytes]:
    """The options of a field that this script has no classes for, such as trestle's, by field
    number: protobuf keeps them as unknown fields, in the field's serialized options."""
    data = f.options.SerializeToString()
    out: dict[int, int | bytes] = {}
    at = 0
    while at < len(data):
        key, at = varint(data, at)
        number, wire = key >> 3, key & 7
        if wire == 0:
            out[number], at = varint(data, at)
        elif wire == 2:
            size, at = varint(data, at)
            out[number] = data[at : at + size]
            at += size
        elif wire == 1:
            at += 8
        elif wire == 5:
            at += 4
        else:
            break
    return out


def response_header(f: descriptor_pb2.FieldDescriptorProto) -> str | None:
    """The response header that a field of a response comes from, if any."""
    value = trestle_options(f).get(RESPONSE_HEADER)
    return value.decode() if isinstance(value, bytes) else None


def raw_body(f: descriptor_pb2.FieldDescriptorProto) -> bool:
    """Whether a field is the HTTP body as raw bytes."""
    return bool(trestle_options(f).get(RAW_BODY))


BINARY = {
    "application/octet-stream": {"schema": {"type": "string", "format": "binary"}}
}


def comment(locations: dict[tuple[int, ...], str], path: tuple[int, ...]) -> str:
    text = locations.get(path, "").strip("\n")
    return "\n".join(line.removeprefix(" ") for line in text.splitlines()).strip()


class Api:
    def __init__(self, descriptor_set: Path) -> None:
        self.messages: dict[str, Message] = {}
        self.enums: dict[str, Enum] = {}
        self.services: list[
            tuple[str, descriptor_pb2.ServiceDescriptorProto, str, list[str]]
        ] = []
        fds = descriptor_pb2.FileDescriptorSet.FromString(descriptor_set.read_bytes())
        for f in fds.file:
            locations = {
                tuple(loc.path): loc.leading_comments
                for loc in f.source_code_info.location
            }
            for i, m in enumerate(f.message_type):
                self.walk(f.package, m, (4, i), "", locations)
            for i, e in enumerate(f.enum_type):
                self.walk_enum(f.package, e, (5, i), "", locations)
            if f.package.startswith("unitycatalog."):
                for si, service in enumerate(f.service):
                    method_comments = [
                        comment(locations, (6, si, 2, mi))
                        for mi in range(len(service.method))
                    ]
                    self.services.append(
                        (
                            f.package,
                            service,
                            comment(locations, (6, si)),
                            method_comments,
                        )
                    )
        # A schema per message and enum, by its simple name; with the package where two share it.
        names: dict[str, list[str]] = {}
        for full, obj in [*self.messages.items(), *self.enums.items()]:
            names.setdefault(obj.name, []).append(full)
        self.schema_names = {}
        for name, fulls in names.items():
            for full in fulls:
                package = full.lstrip(".").split(".")[1]
                self.schema_names[full] = (
                    name if len(fulls) == 1 else f"{package}.{name}"
                )

    def walk(self, package, m, path, prefix, locations):
        full = f".{package}.{prefix}{m.name}"
        self.messages[full] = Message(
            full,
            f"{prefix}{m.name}",
            package,
            m,
            comment(locations, path),
            {i: comment(locations, (*path, 2, i)) for i in range(len(m.field))},
        )
        for i, nested in enumerate(m.nested_type):
            self.walk(package, nested, (*path, 3, i), f"{prefix}{m.name}.", locations)
        for i, e in enumerate(m.enum_type):
            self.walk_enum(package, e, (*path, 4, i), f"{prefix}{m.name}.", locations)

    def walk_enum(self, package, e, path, prefix, locations):
        full = f".{package}.{prefix}{e.name}"
        self.enums[full] = Enum(
            full,
            f"{prefix}{e.name}",
            package,
            e,
            comment(locations, path),
            {i: comment(locations, (*path, 2, i)) for i in range(len(e.value))},
        )

    # --- Schemas ------------------------------------------------------------------------

    def ref(self, full_name: str) -> dict:
        return {"$ref": f"#/components/schemas/{self.schema_names[full_name]}"}

    @staticmethod
    def json_name(f: descriptor_pb2.FieldDescriptorProto, package: str) -> str:
        if package in CAMEL_CASE_PACKAGES:
            return f.json_name or lower_camel(f.name)
        # An explicit `json_name`, one that is not the default.
        if f.json_name and f.json_name != lower_camel(f.name):
            return f.json_name
        return f.name

    @staticmethod
    def required(f: descriptor_pb2.FieldDescriptorProto) -> bool:
        return (
            field_behavior_pb2.REQUIRED
            in f.options.Extensions[field_behavior_pb2.field_behavior]
        )

    def field_schema(self, f: descriptor_pb2.FieldDescriptorProto) -> dict:
        if f.type == FDP.TYPE_MESSAGE:
            msg = self.messages.get(f.type_name)
            if msg is not None and msg.proto.options.map_entry:
                return {
                    "type": "object",
                    "additionalProperties": self.field_schema(msg.proto.field[1]),
                }
            single = (
                dict(WELL_KNOWN[f.type_name])
                if f.type_name in WELL_KNOWN
                else self.ref(f.type_name)
            )
        elif f.type == FDP.TYPE_ENUM:
            single = self.ref(f.type_name)
        elif f.type in INTEGERS:
            single = {"type": "integer", "format": INTEGERS[f.type]}
        elif f.type in (FDP.TYPE_DOUBLE, FDP.TYPE_FLOAT):
            single = {"type": "number"}
        elif f.type == FDP.TYPE_BOOL:
            single = {"type": "boolean"}
        elif f.type == FDP.TYPE_BYTES:
            single = {"type": "string", "contentEncoding": "base64"}
        else:
            single = {"type": "string"}
        if f.label == FDP.LABEL_REPEATED:
            return {"type": "array", "items": single}
        return single

    def object_schema(
        self, msg: Message, fields: list[descriptor_pb2.FieldDescriptorProto]
    ) -> dict:
        properties = {}
        required = []
        for f in fields:
            schema = self.field_schema(f)
            description = msg.field_comments.get(list(msg.proto.field).index(f), "")
            if description:
                # A `$ref` takes no siblings in OpenAPI 3.0; 3.1 allows a description.
                schema = {**schema, "description": description}
            name = self.json_name(f, msg.package)
            properties[name] = schema
            if self.required(f):
                required.append(name)
        schema = {"type": "object", "properties": properties}
        if required:
            schema["required"] = required
        return schema

    def schemas(self) -> dict:
        out = {}
        for full, msg in self.messages.items():
            if (
                not msg.package.startswith("unitycatalog.")
                or msg.proto.options.map_entry
            ):
                continue
            schema = self.object_schema(msg, list(msg.proto.field))
            if msg.comment:
                schema = {"description": msg.comment, **schema}
            out[self.schema_names[full]] = schema
        for full, en in self.enums.items():
            if not en.package.startswith("unitycatalog."):
                continue
            values = [
                (v.name, en.value_comments.get(i, ""))
                for i, v in enumerate(en.proto.value)
                if not (v.number == 0 and v.name.endswith("_UNSPECIFIED"))
            ]
            lines = [en.comment] if en.comment else []
            lines += [
                f"- `{name}`: {text}" if text else f"- `{name}`"
                for name, text in values
            ]
            out[self.schema_names[full]] = {
                "type": "string",
                "enum": [name for name, _ in values],
                "description": "\n".join(lines),
            }
        return dict(sorted(out.items()))

    # --- Operations ---------------------------------------------------------------------

    def operation(
        self, package, service, method, method_comment, verb, path, body
    ) -> dict:
        msg = self.messages[method.input_type]
        path_names = [n.split("=")[0] for n in re.findall(r"{([^}]+)}", path)]
        fields = list(msg.proto.field)
        path_fields = [f for f in fields if f.name in path_names]
        rest = [f for f in fields if f.name not in path_names]
        full_method = f"{package}.{service.name}.{method.name}"
        if body == "*":
            body_fields = fields if full_method in PATH_FIELDS_IN_BODY_METHODS else rest
            query_fields = []
        elif body:
            body_fields = [f for f in rest if f.name == body]
            query_fields = [f for f in rest if f.name != body]
        else:
            body_fields, query_fields = [], rest

        def parameter(f, where):
            description = msg.field_comments.get(fields.index(f), "")
            p = {
                "name": self.json_name(f, msg.package),
                "in": where,
                "required": where == "path" or self.required(f),
                "schema": self.field_schema(f),
            }
            if description:
                p["description"] = description
            return p

        op = {
            "tags": [tag_name(service.name)],
            "operationId": method.name,
            "summary": summary(method_comment) or method.name,
        }
        if method_comment:
            op["description"] = method_comment
        parameters = [parameter(f, "path") for f in path_fields] + [
            parameter(f, "query") for f in query_fields
        ]
        if parameters:
            op["parameters"] = parameters
        if body_fields and all(raw_body(f) for f in body_fields):
            op["requestBody"] = {"required": True, "content": BINARY}
        elif body_fields:
            media = (
                "application/x-www-form-urlencoded"
                if full_method in FORM_METHODS
                else "application/json"
            )
            if len(body_fields) == len(fields):
                schema = self.ref(method.input_type)
            else:
                schema = self.object_schema(msg, body_fields)
            op["requestBody"] = {
                "required": True,
                "content": {media: {"schema": schema}},
            }
        if method.output_type == ".google.protobuf.Empty":
            op["responses"] = {"200": {"description": "Done: an empty body."}}
        else:
            op["responses"] = {"200": self.response(method.output_type)}
        return op

    def response(self, output_type: str) -> dict:
        """The answer of an RPC: its body, and the headers that fields come from."""
        out = self.messages.get(output_type)
        description = (
            summary(out.comment)
            if out and out.comment
            else self.schema_names.get(output_type, "")
        )
        answer: dict = {"description": description or "OK"}
        fields = list(out.proto.field) if out else []
        headers = {name: f for f in fields if (name := response_header(f)) is not None}
        body = [f for f in fields if response_header(f) is None]
        if not headers and not any(raw_body(f) for f in body):
            schema = WELL_KNOWN.get(output_type) or self.ref(output_type)
            answer["content"] = {"application/json": {"schema": schema}}
            return answer
        if any(raw_body(f) for f in body):
            answer["content"] = BINARY
        elif body:
            answer["content"] = {
                "application/json": {"schema": self.object_schema(out, body)}
            }
        answer["headers"] = {
            name: {
                "schema": self.field_schema(f),
                **(
                    {"description": text}
                    if (text := out.field_comments.get(fields.index(f), ""))
                    else {}
                ),
            }
            for name, f in headers.items()
        }
        return answer

    def document(self) -> dict:
        version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"][
            "package"
        ]["version"]
        paths: dict[str, dict] = {}
        tags = []
        groups: dict[str, list[str]] = {}
        for package, service, service_comment, method_comments in self.services:
            name = tag_name(service.name)
            tag = {"name": name}
            if service_comment:
                tag["description"] = service_comment
            tags.append(tag)
            group = next(
                (g for g, services in GROUPS.items() if service.name in services),
                DEFAULT_GROUP,
            )
            groups.setdefault(group, []).append(name)
            for method, method_comment in zip(service.method, method_comments):
                rule = method.options.Extensions[annotations_pb2.http]
                for binding in [rule, *rule.additional_bindings]:
                    verb = binding.WhichOneof("pattern")
                    if verb is None:
                        continue
                    if verb == "custom":
                        # `HEAD`, which `google.api.http` has no field for.
                        verb, path = binding.custom.kind.lower(), binding.custom.path
                    else:
                        path = getattr(binding, verb)
                    # `{name=**}` is a path parameter `{name}`.
                    open_path = re.sub(r"{([^}=]+)=[^}]*}", r"{\1}", path)
                    paths.setdefault(open_path, {})[verb] = self.operation(
                        package,
                        service,
                        method,
                        method_comment,
                        verb,
                        path,
                        binding.body or rule.body,
                    )
        order = [DEFAULT_GROUP, *GROUPS]
        return {
            "openapi": "3.1.0",
            "info": {
                "title": "unitycatalog-rs HTTP APIs",
                "version": version,
                "description": (
                    "The HTTP APIs of the server: the Unity Catalog REST API, the Delta REST API, "
                    "the Iceberg REST catalog, Delta Sharing and the control plane. With "
                    "authorization on, a request carries a token: `Authorization: Bearer <token>`."
                ),
            },
            "servers": [
                {
                    "url": "http://localhost:8080",
                    "description": "A local server (`uc dev`)",
                }
            ],
            "security": [{"bearer": []}],
            "tags": tags,
            "x-tagGroups": [
                {"name": g, "tags": groups[g]} for g in order if g in groups
            ],
            "paths": dict(sorted(paths.items())),
            "components": {
                "securitySchemes": {
                    "bearer": {
                        "type": "http",
                        "scheme": "bearer",
                        "description": "A token of the server or a personal access token.",
                    }
                },
                "schemas": self.schemas(),
            },
        }


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: gen_openapi.py <descriptor set>")
    document = Api(Path(sys.argv[1])).document()
    OUT.write_text(json.dumps(document, indent=2) + "\n")
    operations = sum(len(p) for p in document["paths"].values())
    print(
        f"wrote {OUT.relative_to(ROOT)}: {operations} operations, {len(document['components']['schemas'])} schemas"
    )


if __name__ == "__main__":
    main()
