# prc-rs

"PRC (Product Representation Compact) is a file format that can be used to embed 3D data in a PDF file" [according to Wikipedia](https://en.wikipedia.org/wiki/PRC_(file_format)). prc-rs is a work-in-progress Rust language implementation of the file format. Initial focus is on a parser/reader.

## Project Status

- [x] code generator based on 2014 PRC standard
- [x] PRC double I/O
- [x] Schema evaluator
- [x] Huffman decoding
- [x] parsing compressed arrays
- [ ] parsing PRC_TYPE_TESS_3D_Compressed: vertex and triangle interpretation (https://github.com/pdf-association/pdf-issues/issues/727)
- [ ] parsing PRC_TYPE_TESS_3D_Compressed: normals (https://github.com/pdf-association/pdf-issues/issues/436, https://github.com/pdf-association/pdf-issues/issues/540)
- [ ] parsing PRC_TYPE_TESS_3D_Compressed: color data
- [ ] parsing compressed NURBS
- [ ] parsing AnaFaceTrimLoop
- [x] parsing VertexColors
- [x] parsing PRC_TYPE_SURF_Blend03
- [x] C API for parsing/reading
- [ ] PRC write/output

## PRC Test Data

[Test data](https://github.com/ralovich/prc-db) is currently very scarce, many parts (structures) of the PRC standard are not yet found in existing test data. If you can, please submit new data (sample 3D PDFs) to increase the coverage of the library.

## License

prc-rs is licensed under the MIT License - see the `LICENSE` file for details

## PRC Documentation

The PRC file format documentation is quite scattered, incomplete sometimes contradictory and does not properly detail file version differences.
- SC2N570-PRC-WD [2009 draft standard](https://web.archive.org/web/20091123055411/http://pdf.editme.com/files/PDFE/SC2N570-PRC-WD.pdf)
- ISO 14739-1:2014 [2014 standard](http://www.iso.org/iso/catalogue_detail.htm?csnumber=54948) and [identified issues](https://github.com/pdf-association/pdf-issues/issues?q=prc)
- [Acrobat 9 PRC Format Specification](https://web.archive.org/web/20081202034541/http://livedocs.adobe.com/acrobat_sdk/9/Acrobat9_HTMLHelp/API_References/PRCReference/PRC_Format_Specification/index.html)
- Acrobat SDK 9 for [Mac](http://download.macromedia.com/pub/developer/acrobat/sdk/9/sdk91_v2_mac.dmg) [Win](http://download.macromedia.com/pub/developer/acrobat/sdk/9/sdk91_v2_win.zip)
- Acrobat SDK 10 for [Mac](http://download.macromedia.com/pub/developer/acrobat/sdk/10/sdk100_v1_mac.dmg) [Win](http://download.macromedia.com/pub/developer/acrobat/sdk/10/sdk100_v1_win.zip)
- Acrobat SDK 11 for [Mac](https://download.macromedia.com/pub/developer/acrobat/sdk/11/sdk110_v1_mac.dmg) [Win](https://download.macromedia.com/pub/developer/acrobat/sdk/11/sdk110_v1_win.zip)
