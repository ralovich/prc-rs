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
- [ ] parsing VertexColors
- [ ] parsing PRC_TYPE_SURF_Blend03
- [ ] C API for parsing/reading
- [ ] PRC write/output

## PRC Test Data

Test data (https://github.com/ralovich/prc-db) is currently very scarce, many parts (structures) of the PRC standard are not yet found in existing test data. If you can, please submit new data (sample 3D PDFs) to increase the coverage of the library.

## License

prc-rs is licensed under the MIT License - see the `LICENSE` file for details
