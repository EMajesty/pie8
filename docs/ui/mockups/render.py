#!/usr/bin/env python3
"""Generate PIE8 UI-001 native 320x240 SVG mockups using an original 5x7 font."""

from pathlib import Path
from xml.sax.saxutils import escape

W, H = 320, 240
OUT = Path(__file__).parent
C = {
    "bg": "#071018", "panel": "#101f2b", "major": "#132b38",
    "minor": "#0c1923", "text": "#d8ded5", "muted": "#627483",
    "accent": "#39d6cf", "secondary": "#e574c7", "warning": "#f0bd53",
    "danger": "#f06463", "cursor": "#d8f3e8", "cursor_fg": "#071018",
    "select": "#173f63", "play": "#8ee35b", "green": "#56d477",
}

# Original compact font. Each hexadecimal row mask is five pixels wide.
FONT = {
    " ": [0,0,0,0,0,0,0], "?": [14,17,1,2,4,0,4],
    "A": [14,17,17,31,17,17,17], "B": [30,17,17,30,17,17,30],
    "C": [14,17,16,16,16,17,14], "D": [30,17,17,17,17,17,30],
    "E": [31,16,16,30,16,16,31], "F": [31,16,16,30,16,16,16],
    "G": [14,17,16,23,17,17,15], "H": [17,17,17,31,17,17,17],
    "I": [14,4,4,4,4,4,14], "J": [7,2,2,2,18,18,12],
    "K": [17,18,20,24,20,18,17], "L": [16,16,16,16,16,16,31],
    "M": [17,27,21,21,17,17,17], "N": [17,25,21,19,17,17,17],
    "O": [14,17,17,17,17,17,14], "P": [30,17,17,30,16,16,16],
    "Q": [14,17,17,17,21,18,13], "R": [30,17,17,30,20,18,17],
    "S": [15,16,16,14,1,1,30], "T": [31,4,4,4,4,4,4],
    "U": [17,17,17,17,17,17,14], "V": [17,17,17,17,17,10,4],
    "W": [17,17,17,21,21,21,10], "X": [17,17,10,4,10,17,17],
    "Y": [17,17,10,4,4,4,4], "Z": [31,1,2,4,8,16,31],
    "0": [14,17,19,21,25,17,14], "1": [4,12,4,4,4,4,14],
    "2": [14,17,1,2,4,8,31], "3": [30,1,1,14,1,1,30],
    "4": [2,6,10,18,31,2,2], "5": [31,16,16,30,1,1,30],
    "6": [14,16,16,30,17,17,14], "7": [31,1,2,4,8,8,8],
    "8": [14,17,17,14,17,17,14], "9": [14,17,17,15,1,1,14],
    "-": [0,0,0,31,0,0,0], "+": [0,4,4,31,4,4,0],
    ".": [0,0,0,0,0,12,12], ":": [0,12,12,0,12,12,0],
    "/": [1,2,2,4,8,8,16], "#": [10,31,10,10,31,10,0],
    "*": [0,21,14,31,14,21,0], "[": [14,8,8,8,8,8,14],
    "]": [14,2,2,2,2,2,14], "_": [0,0,0,0,0,0,31],
    "%": [17,2,4,8,17,0,0], "=": [0,31,0,31,0,0,0],
    "<": [2,4,8,16,8,4,2], ">": [8,4,2,1,2,4,8],
    "!": [4,4,4,4,4,0,4], "'": [4,4,2,0,0,0,0],
    "(": [2,4,8,8,8,4,2], ")": [8,4,2,2,2,4,8],
}

class Svg:
    def __init__(self):
        self.e = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" shape-rendering="crispEdges">',
                  f'<rect width="{W}" height="{H}" fill="{C["bg"]}"/>']
    def rect(self, x, y, w, h, color, stroke=None):
        s = f' stroke="{stroke}"' if stroke else ""
        self.e.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{color}"{s}/>')
    def line(self, x1, y1, x2, y2, color):
        self.e.append(f'<path d="M{x1} {y1}H{x2}" stroke="{color}"/>' if y1 == y2 else f'<path d="M{x1} {y1}V{y2}" stroke="{color}"/>')
    def text(self, x, y, value, color=None, scale=1):
        color = color or C["text"]
        for ch in value.upper():
            rows = FONT.get(ch, FONT["?"])
            for ry, bits in enumerate(rows):
                for rx in range(5):
                    if bits & (1 << (4-rx)):
                        self.rect(x + rx*scale, y + ry*scale, scale, scale, color)
            x += 6*scale
    def save(self, name):
        self.e.append(f'<!-- PIE8 UI-001 {escape(name)}: native {W}x{H} -->')
        self.e.append('</svg>')
        data = "\n".join(self.e) + "\n"
        if f'width="{W}" height="{H}"' not in data:
            raise ValueError("non-native mockup")
        (OUT / f"{name}.svg").write_text(data)

def chrome(s, title, object_id="", status="EDIT+DIR CHANGE  SHIFT+DIR VIEW"):
    s.rect(4,4,312,16,C["panel"]); s.text(8,8,title,C["accent"])
    if object_id: s.text(132,8,object_id,C["secondary"])
    s.text(244,8,"120 BPM",C["text"])
    s.rect(4,206,312,18,C["panel"]); s.text(8,211,status[:50],C["muted"])
    s.text(250,228,"O-E-S-P",C["accent"])

def rows(s, headings, data, widths, cursor=(0,0), play=-1, selected=None):
    x0, y0 = 8, 32
    xs=[x0]
    for w in widths: xs.append(xs[-1]+w)
    for i,h in enumerate(headings): s.text(xs[i]+2,23,h,C["muted"])
    for r,row in enumerate(data[:16]):
        y=y0+r*10
        s.rect(4,y-1,312,10,C["major"] if r%4==0 else (C["minor"] if r%2==0 else C["bg"]))
        if selected and selected[0] <= r <= selected[1]: s.rect(xs[selected[2]],y-1,widths[selected[2]],9,C["select"])
        if r==play: s.rect(4,y-1,2,9,C["play"])
        for col,val in enumerate(row):
            if (r,col)==cursor: s.rect(xs[col],y-1,widths[col]-1,9,C["cursor"])
            color=C["cursor_fg"] if (r,col)==cursor else (C["muted"] if set(val)<=set("- ") else C["text"])
            if "*" in val: color=C["warning"]
            s.text(xs[col]+2,y,val,color)
    for x in xs[1:-1]: s.line(x,22,x,192,C["panel"])

def song():
    s=Svg(); chrome(s,"SONG","ROW 04","PLAY ROW 04  CHAIN 12 SHARED")
    heads=["#","DRM","BAS","PAD","LEAD","VOX"]
    vals=[[f"{r:02X}", *(["00","04","08","0C","--"] if r%3 else ["01","05","09","0D","--"])] for r in range(16)]
    vals[4][3]="12*"; rows(s,heads,vals,[22,48,48,48,48,48],(4,3),6,(3,6,1)); s.save("song")

def chain():
    s=Svg(); chrome(s,"CHAIN","12*","SHARED X3  CLONE BEFORE EDIT")
    vals=[]
    for r in range(16): vals.append([f"{r:X}", f"{(0x20+r)%256:02X}" if r<12 else "--", ["+00","+00","+0C","-0C"][r%4]])
    rows(s,["#","PHRASE","TRANSPOSE"],vals,[26,92,96],(5,1),7); s.save("chain")

def phrase():
    s=Svg(); chrome(s,"PHRASE","25*","EDIT NOTE  C4  VELOCITY 72")
    notes=["C-3","---","G-3","---","A#3","---","G-3","OFF","C-4","---","D#4","---","G-4","---","A#4","OFF"]
    vals=[]
    for r,n in enumerate(notes): vals.append([f"{r:X}",n,"03" if n!="---" else "--","72" if n!="---" else "--",("RAT 34" if r==4 else "--- --"), ("PAN 60" if r==10 else "--- --")])
    rows(s,["#","NOTE","IN","VL","FX1","FX2"],vals,[20,38,26,26,64,64],(4,0),8,(4,7,1)); s.save("phrase")

def instrument():
    s=Svg(); chrome(s,"INSTRUMENT","03 SAMPLER","EDIT+PLAY AUDITION  SOURCE 44.1K")
    fields=[("SAMPLE","BREAKS/AMEN 01.WAV"),("ROOT NOTE","C-3"),("PITCH","+00.00 ST"),("START","000120"),("END","048220"),("LOOP","PINGPONG"),("LOOP START","008000"),("LOOP END","032000"),("ATTACK","005 MS"),("DECAY","080 MS"),("SUSTAIN","C0"),("RELEASE","180 MS"),("GAIN","C8"),("PAN","80")]
    for i,(a,b) in enumerate(fields):
        y=28+i*12; s.rect(6,y-2,308,11,C["major"] if i%2==0 else C["bg"]); s.text(10,y,a,C["muted"]); s.text(122,y,b,C["cursor_fg"] if i==5 else C["text"])
        if i==5: s.rect(120,y-2,100,10,C["cursor"]); s.text(122,y,b,C["cursor_fg"])
    s.save("sampler-instrument")

def table():
    s=Svg(); chrome(s,"TABLE","04*","RATE 1 TICK  LOOP 0-F")
    vals=[]
    for r in range(16): vals.append([f"{r:X}",["+00","+07","+0C","-05"][r%4],("VOL C0" if r%4==0 else "--- --"),("PAN 60" if r==8 else "--- --"),("HOP 04" if r==15 else "--- --")])
    rows(s,["#","NOTE","FX1","FX2","FX3"],vals,[22,40,68,68,68],(15,4),5); s.save("table")

def groove():
    s=Svg(); chrome(s,"GROOVE","02","16 STEPS  AVG 6.00 TICKS")
    vals=[[f"{r:X}",str([6,6,5,7][r%4]),"#####" if r%4!=2 else "####",f"{r*6:03d} TICK"] for r in range(16)]
    rows(s,["#","TICKS","PULSE","OFFSET"],vals,[24,54,86,92],(3,1),8); s.save("groove")

def browser():
    s=Svg(); chrome(s,"SAMPLES","/SAMPLES/DRUMS","PLAY AUDITION  EDIT LOAD  OPTION BACK")
    files=[("/..","DIR"),("/BREAKS","DIR"),("KICK 909.WAV","48K M16 24KB"),("SNARE DUST.WAV","48K S24 92KB"),("HIHAT CLOSED.WAV","44K M16 31KB"),("LONG AMEN LOOP.WAV","44K S16 1.3MB"),("TEXTURE 07.WAV","48K S24 2.1MB")]
    for i,(n,m) in enumerate(files):
        y=34+i*18; s.rect(6,y-3,308,16,C["cursor"] if i==5 else (C["major"] if i%2==0 else C["bg"])); s.text(10,y,n,C["cursor_fg"] if i==5 else C["text"]); s.text(190,y,m,C["cursor_fg"] if i==5 else C["muted"])
    s.rect(8,174,304,22,C["panel"]); s.text(12,179,"ARENA 1.3 / 96.0 MB",C["accent"]); s.text(12,189,"DURATION 2.97 S  131072 FRAMES",C["text"]); s.save("sample-browser")

def editor():
    s=Svg(); chrome(s,"SAMPLE EDITOR","AMEN LOOP","NONDESTRUCTIVE RANGE  PLAY AUDITION")
    s.rect(8,32,304,92,C["panel"]); s.line(8,78,312,78,C["muted"])
    for x in range(10,310,2):
        amp=int(8+30*abs(((x*37)%41-20)/20)); s.line(x,78-amp,x,78+amp,C["accent"])
    s.rect(62,33,1,90,C["warning"]); s.rect(255,33,1,90,C["danger"]); s.rect(116,33,1,90,C["secondary"]); s.rect(220,33,1,90,C["secondary"]); s.rect(173,33,2,90,C["play"])
    labels=[("START","000120"),("END","131071"),("LOOP","FORWARD"),("ROOT","C-3"),("GAIN","C8"),("ZOOM","1:256")]
    for i,(a,b) in enumerate(labels):
        x=10+(i%3)*102; y=140+(i//3)*28; s.text(x,y,a,C["muted"]); s.text(x,y+10,b,C["text"])
    s.save("sample-editor")

def mixer():
    s=Svg(); chrome(s,"MIXER","PEAK -3.2 DB","OPTION+PLAY SOLO  OPTION+SHIFT MUTE")
    names=["DRM","BAS","PAD","LED","VOX","FX"]
    for i,n in enumerate(names):
        x=16+i*48; s.text(x,28,n,C["muted"]); s.rect(x,42,10,112,C["panel"])
        h=[92,74,58,84,45,65][i]; s.rect(x+2,152-h,6,h,C["green"] if h<82 else C["warning"]); s.rect(x-4,166,24,2,C["cursor"] if i==1 else C["muted"]); s.text(x-4,176,["C0","B4","98","B8","88","A0"][i],C["text"]); s.text(x-4,190,"C" if i%2 else "L4",C["muted"])
    s.text(10,28,"",C["text"]); s.text(12,198,"MASTER C0   HEADROOM 6DB   CLIPS 0",C["accent"]); s.save("mixer")

def project():
    s=Svg(); chrome(s,"PROJECT","NIGHT TRAIN*","EDIT OPEN  HOLD EDIT CONFIRM WRITE")
    fields=[("NAME","NIGHT TRAIN"),("STATUS","UNSAVED CHANGES"),("TEMPO","120.00 BPM"),("LIVE QUANTIZE","CHAIN"),("NOTE PREVIEW","ON"),("SAVE","DISABLED - READ ONLY"),("LOAD","/PROJECTS"),("NEW PROJECT","CONFIRM"),("UNDO","12 / 64"),("TRACKS","BENCHMARK PENDING")]
    for i,(a,b) in enumerate(fields):
        y=30+i*16; s.rect(6,y-3,308,14,C["major"] if i%2==0 else C["bg"]); s.text(10,y,a,C["muted"]); s.text(130,y,b,C["danger"] if i==5 else (C["cursor_fg"] if i==3 else C["text"]));
        if i==3: s.rect(128,y-3,110,13,C["cursor"]); s.text(130,y,b,C["cursor_fg"])
    s.save("project-settings")

for make in (song, chain, phrase, instrument, table, groove, browser, editor, mixer, project):
    make()
print("generated 10 native 320x240 SVG mockups")
