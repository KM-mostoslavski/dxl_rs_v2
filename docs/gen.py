import html, sys
from xml.sax.saxutils import quoteattr

ROW = 26
SEP = 8
GAP = 30
PAD = 30

ROW_STYLE = "text;align=left;verticalAlign=top;spacingLeft=4;spacingRight=4;overflow=hidden;rotatable=0;points=[[0,0.5],[1,0.5]];portConstraint=eastwest;whiteSpace=wrap;html=1;"
HDR_STYLE = ROW_STYLE + "fontStyle=2;fontColor=#808080;"
SEP_STYLE = "line;strokeWidth=1;fillColor=none;align=left;verticalAlign=middle;spacingTop=-1;spacingBottom=-1;spacingLeft=3;spacingRight=3;rotatable=0;labelPosition=right;points=[];portConstraint=eastwest;strokeColor=inherit;"
EDGE_BASE = "edgeStyle=orthogonalEdgeStyle;rounded=0;orthogonalLoop=1;jettySize=auto;html=1;jumpStyle=arc;"
EDGE_KIND = {
    "impl": "dashed=1;endArrow=block;endFill=0;",
    "comp": "startArrow=diamond;startFill=1;endArrow=none;endFill=0;",
    "assoc": "endArrow=open;endFill=0;",
    "dep": "dashed=1;endArrow=open;endFill=0;",
}
FILL = {
    None: "",
    "Trait": "fillColor=#dae8fc;strokeColor=#6c8ebf;",
    "Enum": "fillColor=#d5e8d4;strokeColor=#82b366;",
    "Module": "fillColor=#f5f5f5;strokeColor=#666666;",
    "Generated": "fillColor=#ffe6cc;strokeColor=#d79b00;",
    "FFI": "fillColor=#f8cecc;strokeColor=#b85450;",
    "Type alias": "fillColor=#e1d5e7;strokeColor=#9673a6;",
    "Marker": "fillColor=#fff2cc;strokeColor=#d6b656;",
    "Macro": "fillColor=#ffe6cc;strokeColor=#d79b00;",
    "External": "fillColor=#f8cecc;strokeColor=#b85450;",
}


def esc(s):
    return html.escape(s, quote=False)


class Diagram:
    def __init__(self):
        self.cells = []
        self.pkgs = []
        self.boxes = {}
        self.n = 0

    def nid(self, p):
        self.n += 1
        return f"{p}{self.n}"

    def cls(self, cid, name, rows, x, y, w=260, stereo=None):
        start = 41 if stereo else 26
        label = (f"<div>&laquo; {esc(stereo)} &raquo;</div>{esc(name)}" if stereo else esc(name))
        out = []
        cy = start
        for r in rows:
            rid = self.nid(cid + "_")
            if r == "---":
                out.append(f'<mxCell id="{rid}" value="" style="{SEP_STYLE}" vertex="1" parent="{cid}"><mxGeometry x="0" y="{cy}" width="{w}" height="{SEP}" as="geometry"/></mxCell>')
                cy += SEP
            else:
                st = ROW_STYLE
                if r.startswith("#"):
                    st, r = HDR_STYLE, r[1:]
                out.append(f'<mxCell id="{rid}" value={quoteattr(esc(r))} style="{st}" vertex="1" parent="{cid}"><mxGeometry x="0" y="{cy}" width="{w}" height="{ROW}" as="geometry"/></mxCell>')
                cy += ROW
        style = ("swimlane;fontStyle=1;align=center;verticalAlign=top;childLayout=stackLayout;horizontal=1;"
                 f"startSize={start};horizontalStack=0;resizeParent=1;resizeParentMax=0;collapsible=1;marginBottom=0;"
                 f"swimlaneHead=0;fillColor=default;html=1;whiteSpace=wrap;{FILL[stereo]}")
        self.cells.append(f'<mxCell id="{cid}" value={quoteattr(label)} style="{style}" vertex="1" parent="1"><mxGeometry x="{x}" y="{y}" width="{w}" height="{cy}" as="geometry"/></mxCell>')
        self.cells.extend(out)
        self.boxes[cid] = (x, y, w, cy)
        return y + cy + GAP

    def column(self, x, y, items, w=260):
        for it in items:
            cid, name, rows = it[:3]
            opts = it[3] if len(it) > 3 else {}
            y = self.cls(cid, name, rows, x, y, opts.get("w", w), opts.get("stereo"))
        return y

    def package(self, label, ids, color="#999999"):
        xs = [self.boxes[i] for i in ids]
        x0 = min(b[0] for b in xs) - PAD
        y0 = min(b[1] for b in xs) - PAD - 20
        x1 = max(b[0] + b[2] for b in xs) + PAD
        y1 = max(b[1] + b[3] for b in xs) + PAD
        pid = self.nid("pkg")
        self.pkgs.append(f'<mxCell id="{pid}" value={quoteattr("<b>" + esc(label) + "</b>")} style="rounded=0;whiteSpace=wrap;html=1;dashed=1;fillColor=none;strokeColor={color};align=left;verticalAlign=top;spacingLeft=8;spacingTop=2;fontSize=14;" vertex="1" parent="1"><mxGeometry x="{x0}" y="{y0}" width="{x1-x0}" height="{y1-y0}" as="geometry"/></mxCell>')

    def note(self, text, x, y, w=300, h=80):
        nid = self.nid("note")
        self.cells.append(f'<mxCell id="{nid}" value={quoteattr(text)} style="shape=note;whiteSpace=wrap;html=1;size=14;align=left;spacingLeft=6;fillColor=#fff2cc;strokeColor=#d6b656;" vertex="1" parent="1"><mxGeometry x="{x}" y="{y}" width="{w}" height="{h}" as="geometry"/></mxCell>')

    def title(self, text, x, y, w=900):
        tid = self.nid("title")
        self.cells.append(f'<mxCell id="{tid}" value={quoteattr(text)} style="text;html=1;fontSize=22;fontStyle=1;align=left;" vertex="1" parent="1"><mxGeometry x="{x}" y="{y}" width="{w}" height="40" as="geometry"/></mxCell>')

    def edge(self, s, t, kind, label=""):
        eid = self.nid("e")
        self.cells.append(f'<mxCell id="{eid}" value={quoteattr(esc(label))} style="{EDGE_BASE}{EDGE_KIND[kind]}" edge="1" parent="1" source="{s}" target="{t}"><mxGeometry relative="1" as="geometry"/></mxCell>')

    def xml(self):
        body = "\n".join(self.pkgs + self.cells)
        return ('<mxfile><diagram name="Page-1"><mxGraphModel adaptiveColors="auto" grid="1" gridSize="10"><root><mxCell id="0"/><mxCell id="1" parent="0"/>\n'
                + body + "\n</root></mxGraphModel></diagram></mxfile>\n")
