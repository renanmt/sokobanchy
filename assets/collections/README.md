# Embedded collections

Reviewed on 2026-10-13 against the user-supplied Sokoban YASC 1.690 files.
Sokobanchy is by **Renan**. These third-party puzzles retain their own authorship
and distribution terms; they are not original Sokobanchy puzzles or relicensed
under the application's GPL notice.

| Collection | Entries | Credits | Permission in the supplied file |
| --- | ---: | --- | --- |
| Skinner | 1,093 | David W. Skinner, sasquatch98930@yahoo.com | Free distribution with author's name and email |
| Crazy Monk | 50 | Jean-Pierre Martel and Matthias Meger, © 2008 | Free distribution retaining copyright and all commentaries |
| GRIGoRusha | 210 | Evgeny Grigoriev; collaborations with Den Gerasimov and Dries De Clercq | Free distribution with author names |
| Holland | 126 | David Holland; one entry credits Aymeric du Peloux | Free distribution retaining copyright; per-puzzle attribution retained |
| Haywood | 276 | Lee J Haywood | Free/commercial distribution of unchanged credited puzzles; explicit GPL-3.0-or-later inclusion terms |
| SokHard | 169 | Lee J Haywood | Same permission and additional terms as Haywood |
| Sladkey | 50 | Rick Sladkey, © 2009 | Free distribution of unchanged credited puzzles; explicit GPL-2.0-or-later inclusion terms |
| YASGen | 10 | Brian Damgaard / YASGen, © 2003 | Free distribution with author's name |

There are 1,984 entries, not necessarily distinct boards across collections.
For example, Haywood and SokHard overlap by 109 boards. Each collection retains
its original order and board content. Identical boards share progress IDs.

The exact original `.sok` bytes are preserved in `sources/`, including all
copyright messages, permissions, author commentaries, and saved solutions.
SHA-256 checksums are recorded in `catalog.json`. The corresponding `.xsb`
files contain unchanged board rows, with titles extracted from the originals.
The `.txt` files retain all prose for the in-app About page, excluding boards,
move recordings, and technical `::` format comments. The complete source files
remain authoritative. No YASC program code, artwork, or sounds are included.

`python3 scripts/generate-collections.py` reproduces the playable files,
About notices, Rust registry, and test solution witnesses from these reviewed
sources. It never reads `shared/`. Tests replay a supplied solution for every
entry and check that importing each original source produces the same board IDs.
Binary releases include this entire directory under `THIRD_PARTY/collections`.

## Files not embedded

| Supplied file | Decision |
| --- | --- |
| `du_Peloux.sok` | The supplied file contains both non-commercial and GPL inclusion wording. The author's [current permission statement](https://aymericdupeloux.wixsite.com/sokoban/post/welcome) also asks users to contact him before reuse. Deferred pending clarification rather than treating credits alone as sufficient. |
| `Yoshio.sok` | Copyright and attribution are present, but no explicit redistribution grant was found in the supplied file or the author's [landing page](https://www.ne.jp/asahi/ai/yoshio/sokoban/). This is not a claim that redistribution is prohibited. |
| `YASC_20_Years.sok` | Credits DrFogh and Anian Wu, without an explicit redistribution grant in the supplied file. |
| `YASGen (with Greek Mythology Puzzles).sok` | Contains separately copyrighted Microsoft/Grolier encyclopedia prose. The simpler `YASGen.sok` has a clear grant and avoids distributing that prose. |

These files remain available to their owner through **Open**; they are not
bundled. YASC's application GPL notice was not assumed to cover every separate
third-party collection. The reference directory remains unchanged.
