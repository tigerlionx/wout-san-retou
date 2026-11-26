#!/usr/bin/env python3
"""Packs the Luau files in src/ into a Roblox Studio place file (WoutSanRetou.rbxlx) that opens with a double click.
No Rojo needed. Run: python3 tools/build_rbxlx.py
The world itself is built by the scripts when the game starts, so the file holds only scripts."""
import itertools
import pathlib
import xml.sax.saxutils as xs

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "src"
OUT = ROOT / "WoutSanRetou.rbxlx"
counter = itertools.count()


def script_item(path: pathlib.Path) -> str:
    name = path.name
    if name.endswith(".server.lua"):
        cls, title = "Script", name[: -len(".server.lua")]
    elif name.endswith(".client.lua"):
        cls, title = "LocalScript", name[: -len(".client.lua")]
    else:
        cls, title = "ModuleScript", name[: -len(".lua")]
    source = path.read_text(encoding="utf-8").replace("]]>", "]]]]><![CDATA[>")
    return (
        f'<Item class="{cls}" referent="RBX{next(counter)}"><Properties>'
        f'<string name="Name">{xs.escape(title)}</string>'
        f'<ProtectedString name="Source"><![CDATA[{source}]]></ProtectedString>'
        f"</Properties></Item>"
    )


def folder_item(name: str, children: str) -> str:
    return (
        f'<Item class="Folder" referent="RBX{next(counter)}"><Properties>'
        f'<string name="Name">{xs.escape(name)}</string></Properties>{children}</Item>'
    )


def service(cls: str, children: str = "") -> str:
    return (
        f'<Item class="{cls}" referent="RBX{next(counter)}"><Properties>'
        f'<string name="Name">{cls}</string></Properties>{children}</Item>'
    )


def scripts_in(directory: pathlib.Path) -> str:
    return "".join(script_item(p) for p in sorted(directory.glob("*.lua")))


body = "".join(
    [
        service("Workspace"),
        service("Lighting"),
        service("ReplicatedStorage", scripts_in(SRC / "ReplicatedStorage")),
        service("ServerStorage", folder_item("WSR", scripts_in(SRC / "ServerStorage" / "WSR"))),
        service("ServerScriptService", scripts_in(SRC / "ServerScriptService")),
        service(
            "StarterPlayer",
            service("StarterPlayerScripts", scripts_in(SRC / "StarterPlayer" / "StarterPlayerScripts"))
            + service("StarterCharacterScripts"),
        ),
    ]
)

OUT.write_text(
    '<roblox xmlns:xmime="http://www.w3.org/2005/05/xmlmime" '
    'xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" '
    'xsi:noNamespaceSchemaLocation="http://www.roblox.com/roblox.xsd" version="4">'
    '<Meta name="ExplicitAutoJoints">true</Meta>' + body + "</roblox>\n",
    encoding="utf-8",
)
print(f"wrote {OUT} ({OUT.stat().st_size} bytes, {next(counter)} items)")
