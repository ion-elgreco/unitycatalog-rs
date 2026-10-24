"""Write the reference of the Python client into the docs site: `just docs-build` runs it.

It reads the package as type checkers do, from its `.pyi` stubs and `.py` modules (no import, so
no compiled extension needed), with griffe, and writes a Markdown page per public module into
docs/reference/python/: the package itself (`Client`, the errors), then a module per API,
with its classes and enums. zensical renders them as the rest of the site; the stylesheet of the
site (stylesheets/extra.css) gives the entries the look of a Sphinx reference: a signature line
per class and method, then its docstring with field lists.

The stubs name types by their private path (`unitycatalog._core.tables.TableInfo`,
`unitycatalog._enums.TableType`); the pages show and link the public one. The PyO3 classes have
no `__init__`: the signature of a class is that of its `__new__`.
"""

from __future__ import annotations

import html
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path

import griffe

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "reference" / "python"
NAV_PREFIX = "reference/python/"
PACKAGE = "unitycatalog"

# Google docstrings: "Returns:" is one description, not "type: description" items.
PARSE_OPTIONS = {"returns_named_value": False, "returns_multiple_items": False}

# Members that are plumbing, not API.
HIDDEN = {
    "__eq__",
    "__repr__",
    "__str__",
    "__reduce__",
    "__hash__",
    "__init__",
    "__new__",
}


@dataclass
class Page:
    slug: str  # file name without .md; the URL is <slug>/
    module: griffe.Module
    title: str


def main() -> None:
    package = griffe.load(
        PACKAGE,
        search_paths=[ROOT / "python" / "client"],
        docstring_parser="google",
        docstring_options=PARSE_OPTIONS,
        allow_inspection=False,
        resolve_aliases=True,
    )
    pages = [Page(PACKAGE, package, PACKAGE)]
    for name in sorted(package.members):
        member = package.members[name]
        if not name.startswith("_") and member.is_module:
            pages.append(Page(name, member, f"{PACKAGE}.{name}"))

    # Where each object is documented: its canonical (private) path -> page and public id.
    # An enum is exported by the package and by the module of its API: it is documented once,
    # on the module's page.
    public: dict[str, tuple[str, str]] = {}
    for page in pages[1:] + pages[:1]:
        for name in page.module.exports or []:
            target = page.module.members[name].final_target
            public.setdefault(target.path, (page.slug, f"{page.title}.{name}"))

    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    (OUT / "index.md").write_text(index_page(pages))
    for page in pages:
        (OUT / f"{page.slug}.md").write_text(Renderer(page, public).render())

    check_nav(["index", *(page.slug for page in pages)])
    print(f"wrote {len(pages) + 1} pages into {OUT.relative_to(ROOT)}")


def check_nav(slugs: list[str]) -> None:
    """Every page is in mkdocs.yml's `nav`: zensical lists in the sidebar only the pages there."""
    nav = (ROOT / "mkdocs.yml").read_text()
    missing = [s for s in slugs if f"{NAV_PREFIX}{s}.md" not in nav]
    if missing:
        sys.exit(
            "mkdocs.yml's nav lacks these pages of the Python client's reference: "
            + ", ".join(f"{NAV_PREFIX}{s}.md" for s in missing)
        )


def first_paragraph(obj: griffe.Object | griffe.Alias) -> str:
    doc = obj.docstring.value if obj.docstring else ""
    return " ".join(doc.split("\n\n")[0].split())


def index_page(pages: list[Page]) -> str:
    rows = "\n".join(
        f"| [`{p.title}`]({p.slug}.md) | {first_paragraph(p.module).split('. ')[0].rstrip('.')} |"
        for p in pages
    )
    return f"""---
title: Python API
---

# Python API

A page per module of the `{PACKAGE}` package: the package itself (`Client`, the errors), then a
module per API, with its classes and enums. The [guide](../../clients/python.md) introduces them.

| Module | Contents |
| --- | --- |
{rows}
"""


class Renderer:
    def __init__(self, page: Page, public: dict[str, tuple[str, str]]) -> None:
        self.page = page
        self.public = public

    # --- Links and types ----------------------------------------------------------------

    def url(self, canonical: str) -> str | None:
        """The link to the documented object at `canonical`: zensical resolves a link of raw HTML,
        as of Markdown, from the page's file."""
        if canonical not in self.public:
            return None
        slug, anchor = self.public[canonical]
        return f"#{anchor}" if slug == self.page.slug else f"{slug}.md#{anchor}"

    def name(self, expr: griffe.ExprName) -> str:
        label = html.escape(expr.name)
        url = self.url(expr.canonical_path)
        return f'<a class="uc-type" href="{url}">{label}</a>' if url else label

    def type(self, expr: object) -> str:
        """A type annotation as HTML: short names, `X | None` for `Optional[X]`, public links."""
        if expr is None:
            return ""
        if isinstance(expr, str):
            return html.escape(expr.replace("typing.", "").replace("builtins.", ""))
        if isinstance(expr, griffe.ExprName):
            return self.name(expr)
        if isinstance(expr, griffe.ExprAttribute):
            return self.type(expr.last)
        if isinstance(expr, griffe.ExprSubscript):
            left = (
                expr.left.last
                if isinstance(expr.left, griffe.ExprAttribute)
                else expr.left
            )
            path = getattr(left, "canonical_path", "")
            if path == "typing.Optional":
                return f"{self.type(expr.slice)} | None"
            if path == "typing.Union":
                return " | ".join(self.type(e) for e in self.elements(expr.slice))
            inner = ", ".join(self.type(e) for e in self.elements(expr.slice))
            return f"{self.type(left)}[{inner}]"
        if isinstance(expr, griffe.ExprBinOp) and expr.operator == "|":
            return f"{self.type(expr.left)} | {self.type(expr.right)}"
        if isinstance(expr, (griffe.ExprTuple, griffe.ExprList)):
            return ", ".join(self.type(e) for e in expr.elements)
        if isinstance(expr, griffe.ExprConstant):
            return html.escape(str(expr.value))
        return html.escape(str(expr).replace("typing.", "").replace("builtins.", ""))

    @staticmethod
    def elements(expr: object) -> list[object]:
        return list(expr.elements) if isinstance(expr, griffe.ExprTuple) else [expr]

    # --- Signatures ---------------------------------------------------------------------

    def signature(
        self,
        kind: str,
        name: str,
        fn: griffe.Function | None,
        *,
        prefix: str = "",
        elide: bool = False,
    ) -> str:
        params: list[str] = []
        if fn is not None and elide:
            params.append('<span class="uc-sig__star">…</span>')
        elif fn is not None:
            seen_star = False
            ps = [p for p in fn.parameters if p.name not in ("self", "cls")]
            for i, p in enumerate(ps):
                if p.kind is griffe.ParameterKind.keyword_only and not seen_star:
                    params.append('<span class="uc-sig__star">*</span>')
                    seen_star = True
                star = {
                    griffe.ParameterKind.var_positional: "*",
                    griffe.ParameterKind.var_keyword: "**",
                }.get(p.kind, "")
                seen_star |= p.kind is griffe.ParameterKind.var_positional
                text = f'<em class="uc-sig__param">{star}{html.escape(p.name)}</em>'
                if p.annotation is not None:
                    text += f": {self.type(p.annotation)}"
                if p.default is not None:
                    text += f' = <span class="uc-sig__default">{html.escape(str(p.default))}</span>'
                params.append(text)
                if p.kind is griffe.ParameterKind.positional_only and (
                    i + 1 == len(ps)
                    or ps[i + 1].kind is not griffe.ParameterKind.positional_only
                ):
                    params.append('<span class="uc-sig__star">/</span>')
        wrap = len(params) > 2
        joined = (
            "".join(f'<span class="uc-sig__p">{p},</span>' for p in params)
            if wrap
            else ", ".join(params)
        )
        returns = ""
        if fn is not None and fn.returns is not None and kind != "class":
            returns = f' <span class="uc-sig__arrow">→</span> {self.type(fn.returns)}'
        kind_html = f'<em class="uc-sig__kind">{kind}</em> ' if kind else ""
        prefix_html = (
            f'<span class="uc-sig__prefix">{html.escape(prefix)}</span>'
            if prefix
            else ""
        )
        params_html = f"({joined})" if fn is not None else ""
        cls = "uc-sig uc-sig--wrap" if wrap else "uc-sig"
        return (
            f'<span class="{cls}">{kind_html}{prefix_html}'
            f'<span class="uc-sig__name">{html.escape(name)}</span>{params_html}{returns}</span>'
        )

    def heading(self, level: int, sig: str, anchor: str, label: str) -> str:
        return f'{"#" * level} {sig} {{ #{anchor} .uc-api data-toc-label="{label}" }}\n'

    # --- Docstrings ---------------------------------------------------------------------

    def docstring(self, obj: griffe.Object, fn: griffe.Function | None = None) -> str:
        if not obj.docstring:
            return ""
        out: list[str] = []
        for section in obj.docstring.parse("google", **PARSE_OPTIONS):
            kind = section.kind.value
            if kind == "text":
                out.append(section.value.strip())
            elif kind == "parameters":
                items = []
                for p in section.value:
                    meta = self.type(p.annotation)
                    # `default` is None for a required parameter, the expression "None" for an
                    # optional one.
                    if p.default is not None and str(p.default) == "None":
                        meta += ", optional"
                    elif p.default is not None:
                        meta += f", default <code>{html.escape(str(p.default))}</code>"
                    items.append((p.name, meta, p.description))
                out.append(self.fields("Parameters", items))
            elif kind == "returns":
                ret = section.value[0]
                desc = ret.description
                # "A table: data in the third layer…" is a description, not "type: description".
                if isinstance(ret.annotation, str) and ret.annotation:
                    desc = f"{ret.annotation}: {desc}"
                ann = (
                    self.type(fn.returns)
                    if fn is not None and fn.returns is not None
                    else ""
                )
                out.append(self.fields("Returns", [("", ann, desc)]))
            elif kind == "raises":
                out.append(
                    self.fields(
                        "Raises",
                        [
                            ("", self.type(e.annotation), e.description)
                            for e in section.value
                        ],
                    )
                )
            elif kind == "examples":
                for sub_kind, text in section.value:
                    out.append(
                        f"```pycon\n{text}\n```"
                        if sub_kind.value == "examples"
                        else text
                    )
            elif kind == "admonition":
                ad = section.value
                body = "\n".join(f"    {line}" for line in ad.description.splitlines())
                out.append(
                    f'!!! {ad.annotation or "note"} "{section.title or ""}"\n{body}'
                )
        return "\n\n".join(s for s in out if s)

    @staticmethod
    def fields(title: str, items: list[tuple[str, str, str]]) -> str:
        lines = []
        for name, meta, desc in items:
            desc = " ".join(line.strip() for line in (desc or "").splitlines())
            head = (
                f'<strong class="uc-fields__name">{html.escape(name)}</strong>'
                if name
                else ""
            )
            if meta:
                head += (
                    f' (<span class="uc-fields__type">{meta}</span>)'
                    if name
                    else f'<span class="uc-fields__type">{meta}</span>'
                )
            lines.append(f"- {head}{' – ' if head and desc else ''}{desc}")
        return (
            f'<div class="uc-fields" markdown>\n<p class="uc-fields__title">{title}</p>\n\n'
            + "\n".join(lines)
            + "\n\n</div>"
        )

    def body(self, text: str) -> str:
        return (
            f'<div class="uc-api__body" markdown>\n\n{text}\n\n</div>\n'
            if text.strip()
            else ""
        )

    # --- Entries ------------------------------------------------------------------------

    def render(self) -> str:
        module = self.page.module
        exports = [(n, module.members[n].final_target) for n in (module.exports or [])]
        if self.page.slug == PACKAGE:
            # Enums are documented on the pages of their modules.
            exports = [(n, t) for n, t in exports if not self.is_enum(t)]
        classes = [(n, t) for n, t in exports if t.is_class and not self.is_enum(t)]
        enums = [(n, t) for n, t in exports if self.is_enum(t)]
        others = [(n, t) for n, t in exports if not t.is_class]

        doc = module.docstring.value.strip() if module.docstring else ""
        out = [
            f"---\ntitle: {self.page.title}\n---\n",
            f"# `{self.page.title}`\n",
            doc + "\n",
        ]
        for label, group in (("Classes", classes), ("Enums", enums)):
            if group:
                rows = "\n".join(
                    f"| [`{n}`](#{self.page.title}.{n}) | {first_paragraph(t) or '&nbsp;'} |"
                    for n, t in group
                )
                out.append(
                    f'<p class="uc-summary__title">{label}</p>\n\n| Name | Description |\n| --- | --- |\n{rows}\n'
                )
        for name, target in classes + enums:
            out.append(self.cls(name, target))
        for name, target in others:
            anchor = f"{self.page.title}.{name}"
            ann = (
                f": {self.type(target.annotation)}"
                if getattr(target, "annotation", None)
                else ""
            )
            sig = f'<span class="uc-sig"><span class="uc-sig__name">{html.escape(name)}</span>{ann}</span>'
            out.append(
                self.heading(2, sig, anchor, name) + self.body(self.docstring(target))
            )
        return "\n".join(out)

    @staticmethod
    def is_enum(obj: griffe.Object) -> bool:
        return obj.is_class and any("Enum" in str(b) for b in obj.bases)

    def cls(self, name: str, target: griffe.Class) -> str:
        anchor = f"{self.page.title}.{name}"
        members = target.members
        ctor = members.get("__init__") or members.get("__new__")
        if self.is_enum(target):
            ctor = None
        sig = self.signature("class", name, ctor, prefix=f"{self.page.title}.")
        parts = [self.docstring(target)]
        bases = [
            b
            for b in target.bases
            if "Enum" not in str(b) and str(b) not in ("object", "Exception")
        ]
        if bases:
            parts.insert(
                0,
                '<p class="uc-api__bases">Bases: '
                + ", ".join(self.type(b) for b in bases)
                + "</p>",
            )
        if self.is_enum(target):
            items = [
                (n, "", first_paragraph(m))
                for n, m in members.items()
                if m.is_attribute and not n.startswith("_")
            ]
            parts.append(self.fields("Members", items))
        props = [
            (n, m)
            for n, m in members.items()
            if m.is_attribute and "property" in m.labels and not n.startswith("_")
        ]
        if props:
            parts.append(
                self.fields(
                    "Attributes",
                    [
                        (n, self.type(m.annotation), first_paragraph(m))
                        for n, m in props
                    ],
                )
            )
        out = [
            self.heading(2, sig, anchor, name)
            + self.body("\n\n".join(p for p in parts if p))
        ]

        methods = {
            n: m
            for n, m in members.items()
            if m.is_function and n not in HIDDEN and not n.startswith("_")
        }
        for mname, method in methods.items():
            if mname.endswith("_async") and mname[: -len("_async")] in methods:
                continue  # shown with its synchronous twin
            m_anchor = f"{anchor}.{mname}"
            m_sig = self.signature("", mname, method)
            text = self.docstring(method, method)
            twin = methods.get(f"{mname}_async")
            if twin is not None:
                # The same parameters: `…` stands for them.
                text += (
                    '\n\n<p class="uc-api__twin">Async: '
                    + self.signature("", twin.name, twin, elide=True)
                    + "</p>"
                )
            out.append(self.heading(3, m_sig, m_anchor, mname) + self.body(text))
        return "\n".join(out)


if __name__ == "__main__":
    main()
