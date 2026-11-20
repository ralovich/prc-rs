# `prc`

"PRC (Product Representation Compact) is a file format that can be used to embed 3D data in a PDF file" [according to Wikipedia](https://en.wikipedia.org/wiki/PRC_(file_format)). `prc` is a Rust language implementation of the file format. `prc` allows parsing PRC files into a well-defined, versioned JSON representation. The goal of the JSON representation is twofold:
1. easily allow further processing in any language/environment and
2. to enable round-tripping to/from obscure binary PRC.

`prc` offers a simple C API for parsing to JSON. In the future `prc` will also expose a simple C API for writing to PRC.

## `prc` Status

- [x] [declarative description](https://github.com/ralovich/prc-rs/blob/main/src/prc.json) of 2014 PRC standard including errata and [code generator](https://github.com/ralovich/prc-rs/blob/main/src/generate_rust)
- [x] PRC extension schema evaluator
- [x] Huffman (de)coding
- [x] parsing compressed arrays
- [x] parsing PRC_TYPE_TESS_3D_Compressed
- [ ] decompression of PRC_TYPE_TESS_3D_Compressed: vertices, normals and triangles
- [ ] decompression of PRC_TYPE_TESS_3D_Compressed: color data
- [ ] decompression of PRC_TYPE_TESS_3D_Compressed: texture coordinates
- [x] parsing compressed NURBS
- [x] parsing AnaFaceTrimLoop
- [x] parsing VertexColors
- [x] parsing PRC_TYPE_SURF_Blend03
- [ ] decompression of compressed B-rep
- [ ] B-rep tessellation
- [x] C API for parsing/reading
- [ ] C API for PRC write/synthesis

## How to parse a PRC from the command line

Build the CMake project under `src/tools/prc2json`. Afterwards run `prc2json input.prc | jq > output.json`.

## How to parse a PRC from C/C++

```cpp
#include <prc.h>

void* allocate(const uint64_t num_bytes)
{
    return new unsigned char[static_cast<size_t>(num_bytes)];
}

std::vector<char> prc = ...; // raw bytes from a PRC file
uint64_t json_returned_bytes = 0;
unsigned char* json;

int rv = prc_parse_to_json(prc.size(),
                           reinterpret_cast<unsigned char*>(prc.data()),
                           allocate,
                           &json,
                           &json_returned_bytes,
                           nullptr);
```

## How to parse a PRC from Rust

```rust
use prc;

let prc: Vec<u8> = ...; // raw bytes from a PRC file
let opts: prc::common::ParsingOptions {...};
if let Ok(parsed_prc) = prc::common::prc_describe(&prc, "file-base-name.prc", &opts) {
  let json = serde_json::to_string(&parsed_prc)?;
}
```

## PRC Test Data

[Test data](https://github.com/ralovich/prc-db) is currently incomplete, many parts (structures) of the PRC standard are not yet found in existing test data. If you can, please submit new data (sample 3D PDFs) to increase the coverage of the library. The script [extract3d.sh](https://github.com/ralovich/prc-rs/blob/main/scripts/extract3d.sh) may be used to find and dump PRC and U3D streams from a PDF file.

## License

`prc` is licensed under the MIT License - see the `LICENSE` file for details.

For commercial support and consulting reach out to the main author, Kristóf Ralovich.

## PRC Documentation

The PRC file format documentation is quite scattered, incomplete, sometimes contradictory and does not properly detail file version differences.

- SC2N570-PRC-WD [2009 draft standard](https://web.archive.org/web/20091123055411/http://pdf.editme.com/files/PDFE/SC2N570-PRC-WD.pdf)
- ISO 14739-1:2014 [2014 standard](http://www.iso.org/iso/catalogue_detail.htm?csnumber=54948) and [identified issues](https://github.com/pdf-association/pdf-issues/issues?q=prc)
- [Acrobat 9 PRC Format Specification](https://web.archive.org/web/20081202034541/http://livedocs.adobe.com/acrobat_sdk/9/Acrobat9_HTMLHelp/API_References/PRCReference/PRC_Format_Specification/index.html)
- Acrobat SDK 9 for [Mac](http://download.macromedia.com/pub/developer/acrobat/sdk/9/sdk91_v2_mac.dmg) [Win](http://download.macromedia.com/pub/developer/acrobat/sdk/9/sdk91_v2_win.zip)
- Acrobat SDK 10 for [Mac](http://download.macromedia.com/pub/developer/acrobat/sdk/10/sdk100_v1_mac.dmg) [Win](http://download.macromedia.com/pub/developer/acrobat/sdk/10/sdk100_v1_win.zip)
- Acrobat SDK 11 for [Mac](https://download.macromedia.com/pub/developer/acrobat/sdk/11/sdk110_v1_mac.dmg) [Win](https://download.macromedia.com/pub/developer/acrobat/sdk/11/sdk110_v1_win.zip)
- [US pat. 8.207,965](https://patents.google.com/patent/US8207965B2/en) on compressed tessellation
- [Acrobat Pro Extended 9](http://trials.adobe.com/Applications/Acrobat/APEX/9/APEX9_Win_WEB_WWEFG.7z)
