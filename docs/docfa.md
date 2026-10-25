# DXF and DWG compatible with DOCFA (Italian cadastre)

> **Status:** Save As has two file types, *DXF compatible with DOCFA* and *DWG compatible with
> DOCFA* (also in File and Options ▸ Open and Save); SAVEAS / EXPORT `docfa: true`;
> `cadcraft-cli run … --docfa`. Writer: `crates/io/src/dxf_r12.rs` (`docfa_drawing`).

DOCFA is the Agenzia delle Entrate's program for cadastral declarations (*Documenti Catasto
Fabbricati*). It imports a floor plan (*planimetria*) from a DXF or DWG file and, from closed
polylines on the layer `DOCFA_POLIGONI` of a DXF, computes the surfaces of the unit.

## What DOCFA takes: the sources

- **The Agenzia's software page** ([Software Docfa 4.00.5](https://www.agenziaentrate.gov.it/portale/software-docfa-4005),
  version 4.00.5 of 03/07/2019) names DXF and DWG as the formats of the import, which needs the
  library file `ADINIT.DAT` in the Windows system folder (`System32`, `SysWOW64` on 64-bit
  Windows). It states no version.
- **The Agenzia's own DXF templates** (`modelli.zip` on that page) are AutoCAD **R12** DXF
  (`$ACADVER` AC1009), in metres, each a closed polyline on the layer `RIQUADRO` with its
  lower-left corner at **0,0**: the printable area of the sheet at the scale.

  | Template | Frame (metres) | Sheet area at scale |
  |---|---|---|
  | A4 1:100 | 19.5 × 23.5 | 195 × 235 mm |
  | A4 1:200 | 39 × 47 | 195 × 235 mm |
  | A4 1:500 | 97.5 × 117.5 | 195 × 235 mm |
  | A3 1:100 | 26 × 40.5 | 260 × 405 mm |
  | A3 1:200 | 52 × 81 | 260 × 405 mm |
  | A3 1:500 | 130 × 202.5 | 260 × 405 mm |

- **Practitioners** report that Docfa 4 loads DWG saved as **AutoCAD 2000 or earlier** and not
  newer ones, that DXF 2000 works too, that the polygon layer is read from DXF only (from a DWG
  DOCFA computes the surfaces itself), and that some DXF 2000 files came in at the wrong scale
  ([GeoLIVE](https://www.geolive.org/guide/procedure-e-metodologie/docfa/docfa-4-come-importare-i-dwg-e-dxf-21123/),
  [professionearchitetto.it](https://www.professionearchitetto.it/news/notizie/30244/Planimetria-DoCFa-come-impostarla-e-tutte-le-regole-da-rispettare),
  [architetticampagna](https://architetticampagna.blogspot.com/2013/01/docfa-trucchi-e-suggerimenti.html)).

## What CADCraft writes

One choice per format, no version to pick:

| Save As file type | File | Polygon layer |
|---|---|---|
| **DXF compatible with DOCFA** | DXF, AutoCAD 2000 (AC1015) | yes (`DOCFA_POLIGONI`) |
| **DWG compatible with DOCFA** | DWG, AutoCAD 2000 (AC1015) | no: DOCFA computes the surfaces |

Both follow the templates' conventions: metres, the frame and everything else where the drawing
has them (draw the plan inside the Agenzia's frame, corner at 0,0; the export warns when the
drawing extends below or left of it), and **no declared drawing unit** (`$INSUNITS` 0), as R12
has none: a reader that converts a declared unit (metres into its own millimetres) would scale
the plan, which is what the wrong-scale reports point to. Checked with the A4 1:200 template: the
DXF passes ezdxf's audit with no error and keeps the 39 × 47 frame at 0,0, and the DWG reads back
with the same contents.

R12 stays available as an ordinary DXF version (Save As ▸ Drawing (DXF) ▸ R12, `version: "R12"`,
`--save-version R12`) for other old programs, and `saveas {"docfa": true, "version": "R12"}`
writes the DOCFA profile in the templates' own R12 format if a DOCFA installation refuses the
2000 file.

## How to export

- **Desktop:** File ▸ Save as DXF compatible with DOCFA… (or DWG), or File ▸ Save As… and the
  file type *DXF compatible with DOCFA* / *DWG compatible with DOCFA*, or Options ▸ Open and Save ▸
  DOCFA. Pick the drawing's unit (m, cm, mm) ▸ Save…. The suggested name is
  `<drawing>-docfa.dxf`. The export does not rename the drawing: keep saving the drawing itself
  and export for DOCFA when a sheet is ready.
- **Command line / scripts / MCP:** `saveas {"path": "plan-docfa.dxf", "docfa": true, "unit": "cm"}`,
  or a `.dwg` path for the DWG; `export` takes the same. `scale` takes a number instead of `unit`
  (drawing units × scale = metres). The result's `notes` and the command line say what was
  converted or left out.
- **Headless:** `cadcraft-cli run plan.dwg --save plan-docfa.dxf --docfa --unit cm`, or
  `--save plan-docfa.dwg`.

## What the DOCFA profile writes

| In the drawing | In the file |
|---|---|
| Model space | Written. Paper-space layouts are left out (R12 has no layouts). |
| Block references (doors, windows, north arrow, title block) | Exploded into their lines, arcs and text, nested blocks too (DOCFA guides: *eliminare i blocchi*). Attributes become TEXT. |
| Polylines | Kept with their bulges and widths (old-style POLYLINE / VERTEX in R12). |
| MTEXT | One TEXT per line, placed by the attachment point; formatting dropped. |
| Ellipses, splines, helixes, regions, multilines, leaders, multileaders | Polylines (tessellated), lines and text. |
| Dimensions, tables | Their lines, arrowheads (SOLID) and text. |
| Hatches, images, wipeouts, viewports, rays, xlines | Left out (*senza retini*). |
| Objects on layers that are off or frozen | Left out (the guides say to switch off the frame and polygon layers you don't want). |
| Layers | All written, white (7) and CONTINUOUS: DOCFA ignores layer colours. Names become R12 names (upper case, `A–Z 0–9 $ - _`, 31 characters). |
| Colours | Only on the polylines of layer **DOCFA_POLIGONI** (group 62, the object's own or its layer's colour, as an ACI). Everything else is ByLayer, so DOCFA draws it in its own colour. |
| Text | Transliterated to ASCII: `è` → `e`, `°` → `%%d`, `±` → `%%p`, `Ø` → `%%c`; other characters → `?`. Old readers show `\U+00E8` otherwise. |
| Units | Multiplied by `scale` (`unit` m = 1, cm = 0.01, mm = 0.001): DOCFA takes **metres**. |
| Header | `$ACADVER` AC1015 (AC1009 for R12), `$INSUNITS` 0, `$EXTMIN/$EXTMAX` of what was written, `$INSBASE` 0, `$LUNITS` 2. |
| Tables | VPORT (`*ACTIVE` showing everything), LTYPE (CONTINUOUS), LAYER, STYLE (STANDARD, `txt`). No DIMSTYLE, UCS, VIEW or APPID records, no CLASSES or OBJECTS sections. |

The plain R12 version keeps block references as INSERT (with ATTRIB), writes the drawing's own
linetypes, text styles and entity colours, writes hatch pattern lines as LINEs (solid fills as
their boundary polylines), and keeps the drawing's `$INSBASE`, `$LIMMIN/$LIMMAX` and header
variables. Everything else is the same conversion.

## DOCFA checks

The export reports, on the command line and in the result's `notes`:

- how many surface polygons layer DOCFA_POLIGONI holds, and how many are **not closed** (DOCFA
  needs closed polylines: *tutte le polilinee devono essere rigorosamente chiuse*);
- objects on DOCFA_POLIGONI that aren't polylines;
- polygons whose colour is outside 1–8 (the area classes below);
- a drawing extending left of or below the origin: DOCFA expects the sheet frame's lower-left
  corner at **0,0**;
- a drawing spanning more than 1000 units: probably not in metres yet (use `unit`).

## Preparing the drawing (from the DOCFA guides)

- **One CAD unit = one metre** in the file: draw in metres, or export with `unit: "cm"` / `"mm"`.
- **Sheet frame at the origin:** the frame's lower-left corner at 0,0. At 1:200 the frames are
  A4 = 39 × 47 units, A3 = 52 × 81 units (the Agenzia's own DXF templates come in A4/A3 at 1:100,
  1:200 and 1:500). Put the frame on its own layer (`RIQUADRO` / `CART`) and switch that layer off
  before exporting, or leave it on if your DOCFA version expects it.
- **Layer `0` for the plan** (walls, partitions, doors, stairs, texts); plain lines, no
  lineweights, no hatching, no furniture.
- **Layer `DOCFA_POLIGONI`** for the surface polygons, one closed polyline per area, each in the
  colour of its class (DXF group 62):

  | Class | Colour (ACI) | Meaning |
  |---|---|---|
  | A | 1 red | main rooms and their direct accessories |
  | B | 2 yellow | indirect-service accessories, communicating |
  | C | 3 green | indirect-service accessories, non-communicating |
  | D | 4 cyan | balconies, terraces, loggias, communicating |
  | E | 5 blue | balconies, terraces, loggias, non-communicating |
  | F | 6 magenta | exclusive outdoor areas (courtyards, gardens) |
  | G | 7 white | spaces not relevant for the calculation |
  | A2 | 8 grey | accessories serving C/1 and C/6 units directly |

  Guides differ on the exact class ↔ colour table between DOCFA versions; check the DOCFA
  release notes and the Agenzia's templates for the version you file with.
- **Required contents:** the scale, the height of the rooms in metres (rounded to 5 cm), a north
  symbol outside the plan, the floor (S1, S2 for basements), the accesses, exclusive courtyards.
- **Text:** plain words (`cucina`/`K`, `bagno`, `ripostiglio`…), no multiline text (exploded by
  the export anyway), no neighbours' names (use cadastral identifiers or *altra u.i.u.*).

## Sources

Italian practitioners' guides; DOCFA itself is closed and undocumented beyond its release notes:

- GeoLIVE, *Planimetrie CAD… come inserirle in DocFa* — version 12, units, frames, the
  DOCFA_POLIGONI layer and its colours.
- professionearchitetto.it, *Planimetria DoCFa, come impostarla e tutte le regole da rispettare* —
  layers, colours by object, required contents, what makes a filing refused.
- forum.topgeometri.it, *Problema con importazione file dxf in docfa* — start from a clean R12
  file, plain lines, no hatches, no blocks, the title block at 0,0.
- Agenzia delle Entrate, *Software Docfa* downloads — the DXF templates (A4/A3, 1:100/1:200/1:500).

No DOCFA file, template or code was copied: the format comes from Autodesk's public DXF
Reference (R12 section) and CADCraft's own synthetic tests (`crates/io/src/dxf_r12/tests.rs`).
