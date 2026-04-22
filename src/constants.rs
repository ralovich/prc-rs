// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#![allow(unused, nonstandard_style)]

use modular_bitfield::bitfield;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::mem;

extern crate static_assertions as sa;

pub enum PrcSectionKind {
    Global,
    Tree,
    Tessellation,
    Geometry,
    ExtraGeometry,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, TryFromPrimitive)]
#[repr(u32)]
pub enum PrcType {
    PRC_TYPE_ROOT = 0,

    PRC_TYPE_ROOT_PRCBase = 1,
    PRC_TYPE_ROOT_PRCBaseWithGraphics = 2,

    PRC_TYPE_CRV = 10,
    PRC_TYPE_SURF = 75,
    PRC_TYPE_TOPO = 140,
    PRC_TYPE_TESS = 170,
    PRC_TYPE_MISC = 200,
    PRC_TYPE_RI = 230,
    PRC_TYPE_ASM = 300,
    PRC_TYPE_MKP = 500,
    PRC_TYPE_GRAPH = 700,
    PRC_TYPE_MATH = 900,

    PRC_TYPE_CRV_Base = 11,
    PRC_TYPE_CRV_Blend02Boundary = 12,
    PRC_TYPE_CRV_NURBS = 13,
    PRC_TYPE_CRV_Circle = 14,
    PRC_TYPE_CRV_Composite = 15,
    PRC_TYPE_CRV_OnSurf = 16,
    PRC_TYPE_CRV_Ellipse = 17,
    PRC_TYPE_CRV_Equation = 18,
    PRC_TYPE_CRV_Helix = 19,
    PRC_TYPE_CRV_Hyperbola = 20,
    PRC_TYPE_CRV_Intersection = 21,
    PRC_TYPE_CRV_Line = 22,
    PRC_TYPE_CRV_Offset = 23,
    PRC_TYPE_CRV_Parabola = 24,
    PRC_TYPE_CRV_PolyLine = 25,
    PRC_TYPE_CRV_Transform = 26,

    PRC_TYPE_SURF_Base = 76,
    PRC_TYPE_SURF_Blend01 = 77,
    PRC_TYPE_SURF_Blend02 = 78,
    PRC_TYPE_SURF_Blend03 = 79,
    PRC_TYPE_SURF_NURBS = 80,
    PRC_TYPE_SURF_Cone = 81,
    PRC_TYPE_SURF_Cylinder = 82,
    PRC_TYPE_SURF_Cylindrical = 83,
    PRC_TYPE_SURF_Offset = 84,
    PRC_TYPE_SURF_Pipe = 85,
    PRC_TYPE_SURF_Plane = 86,
    PRC_TYPE_SURF_Ruled = 87,
    PRC_TYPE_SURF_Sphere = 88,
    PRC_TYPE_SURF_Revolution = 89,
    PRC_TYPE_SURF_Extrusion = 90,
    PRC_TYPE_SURF_FromCurves = 91,
    PRC_TYPE_SURF_Torus = 92,
    PRC_TYPE_SURF_Transform = 93,
    PRC_TYPE_SURF_Blend04 = 94,

    PRC_TYPE_TOPO_Context = 141,
    PRC_TYPE_TOPO_Item = 142,
    PRC_TYPE_TOPO_MultipleVertex = 143,
    PRC_TYPE_TOPO_UniqueVertex = 144,
    PRC_TYPE_TOPO_WireEdge = 145,
    PRC_TYPE_TOPO_Edge = 146,
    PRC_TYPE_TOPO_CoEdge = 147,
    PRC_TYPE_TOPO_Loop = 148,
    PRC_TYPE_TOPO_Face = 149,
    PRC_TYPE_TOPO_Shell = 150,
    PRC_TYPE_TOPO_Connex = 151,
    PRC_TYPE_TOPO_Body = 152,
    PRC_TYPE_TOPO_SingleWireBody = 153,
    PRC_TYPE_TOPO_BrepData = 154,
    PRC_TYPE_TOPO_SingleWireBodyCompress = 155,
    PRC_TYPE_TOPO_BrepDataCompress = 156,
    PRC_TYPE_TOPO_WireBody = 157,

    PRC_TYPE_TESS_Base = 171,
    PRC_TYPE_TESS_3D = 172,
    PRC_TYPE_TESS_3D_Compressed = 173,
    PRC_TYPE_TESS_Face = 174,
    PRC_TYPE_TESS_3D_Wire = 175,
    PRC_TYPE_TESS_Markup = 176,

    PRC_TYPE_MISC_Attribute = 201,
    PRC_TYPE_MISC_CartesianTransformation = 202,
    PRC_TYPE_MISC_EntityReference = 203,
    PRC_TYPE_MISC_MarkupLinkedItem = 204,
    PRC_TYPE_MISC_ReferenceOnPRCBase = 205,
    PRC_TYPE_MISC_ReferenceOnTopology = 206,
    PRC_TYPE_MISC_GeneralTransformation = 207,

    PRC_TYPE_RI_RepresentationItem = 231,
    PRC_TYPE_RI_BrepModel = 232,
    PRC_TYPE_RI_Curve = 233,
    PRC_TYPE_RI_Direction = 234,
    PRC_TYPE_RI_Plane = 235,
    PRC_TYPE_RI_PointSet = 236,
    PRC_TYPE_RI_PolyBrepModel = 237,
    PRC_TYPE_RI_PolyWire = 238,
    PRC_TYPE_RI_Set = 239,
    PRC_TYPE_RI_CoordinateSystem = 240,

    PRC_TYPE_ASM_ModelFile = 301,
    PRC_TYPE_ASM_FileStructure = 302,
    PRC_TYPE_ASM_FileStructureGlobals = 303,
    PRC_TYPE_ASM_FileStructureTree = 304,
    PRC_TYPE_ASM_FileStructureTessellation = 305,
    PRC_TYPE_ASM_FileStructureGeometry = 306,
    PRC_TYPE_ASM_FileStructureExtraGeometry = 307,
    PRC_TYPE_ASM_ProductOccurrence = 310,
    PRC_TYPE_ASM_PartDefinition = 311,
    PRC_TYPE_ASM_Filter = 320,

    PRC_TYPE_MKP_View = 501,
    PRC_TYPE_MKP_Markup = 502,
    PRC_TYPE_MKP_Leader = 503,
    PRC_TYPE_MKP_AnnotationItem = 504,
    PRC_TYPE_MKP_AnnotationSet = 505,
    PRC_TYPE_MKP_AnnotationReference = 506,

    PRC_TYPE_GRAPH_Style = 701,
    PRC_TYPE_GRAPH_Material = 702,
    PRC_TYPE_GRAPH_Picture = 703,
    PRC_TYPE_GRAPH_TextureApplication = 711,
    PRC_TYPE_GRAPH_TextureDefinition = 712,
    PRC_TYPE_GRAPH_TextureTransformation = 713,
    PRC_TYPE_GRAPH_LinePattern = 721,
    PRC_TYPE_GRAPH_FillPattern = 722,
    PRC_TYPE_GRAPH_DottingPattern = 723,
    PRC_TYPE_GRAPH_HatchingPattern = 724,
    PRC_TYPE_GRAPH_SolidPattern = 725,
    PRC_TYPE_GRAPH_VPicturePattern = 726,
    PRC_TYPE_GRAPH_AmbientLight = 731,
    PRC_TYPE_GRAPH_PointLight = 732,
    PRC_TYPE_GRAPH_DirectionalLight = 733,
    PRC_TYPE_GRAPH_SpotLight = 734,
    PRC_TYPE_GRAPH_SceneDisplayParameters = 741,
    PRC_TYPE_GRAPH_Camera = 742,

    PRC_TYPE_MATH_FCT_1D = 901,
    PRC_TYPE_MATH_FCT_1D_Polynom = 902,
    PRC_TYPE_MATH_FCT_1D_Trigonometric = 903,
    PRC_TYPE_MATH_FCT_1D_Fraction = 904,
    PRC_TYPE_MATH_FCT_1D_ArctanCos = 905,
    PRC_TYPE_MATH_FCT_1D_Combination = 906,
    PRC_TYPE_MATH_FCT_3D = 910,
    PRC_TYPE_MATH_FCT_3D_Linear = 911,
    PRC_TYPE_MATH_FCT_3D_NonLinear = 912,
}
impl fmt::Display for PrcType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} ({})", self, *self as u32)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum PrcGraphicsBehavior {
    PRC_GRAPHICS_Show = 0x0001,
    PRC_GRAPHICS_ChildHeritShow = 0x0002,
    PRC_GRAPHICS_FatherHeritShow = 0x0004,
    PRC_GRAPHICS_ChildHeritColor = 0x0008,
    PRC_GRAPHICS_ParentHeritColor = 0x0010,
    PRC_GRAPHICS_ChildHeritLayer = 0x0020,
    PRC_GRAPHICS_ParentHeritLayer = 0x0040,
    PRC_GRAPHICS_ChildHeritTransparency = 0x0080,
    PRC_GRAPHICS_ParentHeritTransparency = 0x0100,
    PRC_GRAPHICS_ChildHeritLinePattern = 0x0200,
    PRC_GRAPHICS_ParentHeritLinePattern = 0x0400,
    PRC_GRAPHICS_ChildHeritLineWidth = 0x0800,
    PRC_GRAPHICS_ParentHeritLineWidth = 0x1000,
    PRC_GRAPHICS_Removed = 0x2000,
}

#[bitfield]
#[repr(u16)]
#[derive(Debug)]
pub struct PrcGraphicsBehaviorBitField {
    PRC_GRAPHICS_Show: bool,
    PRC_GRAPHICS_ChildHeritShow: bool,
    PRC_GRAPHICS_FatherHeritShow: bool,
    PRC_GRAPHICS_ChildHeritColor: bool,
    PRC_GRAPHICS_ParentHeritColor: bool,
    PRC_GRAPHICS_ChildHeritLayer: bool,
    PRC_GRAPHICS_ParentHeritLayer: bool,
    PRC_GRAPHICS_ChildHeritTransparency: bool,
    PRC_GRAPHICS_ParentHeritTransparency: bool,
    PRC_GRAPHICS_ChildHeritLinePattern: bool,
    PRC_GRAPHICS_ParentHeritLinePattern: bool,
    PRC_GRAPHICS_ChildHeritLineWidth: bool,
    PRC_GRAPHICS_ParentHeritLineWidth: bool,
    PRC_GRAPHICS_Removed: bool,
    #[skip]
    unused: modular_bitfield::specifiers::B2,
}

#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum PrcTransformation {
    PRC_TRANSFORMATION_Identity = 0x00,
    PRC_TRANSFORMATION_Translate = 0x01,
    PRC_TRANSFORMATION_Rotate = 0x02,
    PRC_TRANSFORMATION_Mirror = 0x04,
    PRC_TRANSFORMATION_Scale = 0x08,
    PRC_TRANSFORMATION_NonUniformScale = 0x10,
    PRC_TRANSFORMATION_NonOrtho = 0x20,
    PRC_TRANSFORMATION_Homogeneous = 0x40,
}
#[bitfield]
#[repr(u8)]
#[derive(Debug)]
pub struct PrcTransformationBitField {
    //PRC_TRANSFORMATION_Identity = 0x00,
    PRC_TRANSFORMATION_Translate: bool, // 0x01
    PRC_TRANSFORMATION_Rotate: bool,    // 0x02
    PRC_TRANSFORMATION_Mirror: bool,
    PRC_TRANSFORMATION_Scale: bool,
    PRC_TRANSFORMATION_NonUniformScale: bool,
    PRC_TRANSFORMATION_NonOrtho: bool,
    PRC_TRANSFORMATION_Homogeneous: bool,
    #[skip]
    unused: modular_bitfield::specifiers::B1,
}

#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum TextureMappingType {
    Unknown = 0,
    Stored = 1,
    Parametric = 2,
    Operator = 3,
}

#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum TextureFunction {
    Unknown = 0,
    Modulate = 1,
    Replace = 2,
    Blend = 3,
    Decal = 4,
}

#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum TextureApplicationMode {
    Undefined = 0x0000,
    Lighting = 0x0001,
    AlphaTest = 0x0002,
    TextureColor = 0x0004,
}

#[bitfield]
#[repr(u8)]
#[derive(Debug)]
pub struct PrcBodyBoundingBoxBehaviorBitField {
    pub PRC_BODY_BBOX_Evaluation: bool, // 0x01
    pub PRC_BODY_BBOX_Precise: bool,    // 0x02
    pub PRC_BODY_BBOX_CADData: bool,    // 0x04
    #[skip]
    unused: modular_bitfield::specifiers::B5,
}

#[repr(u32)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum PrcCompressedFaceType {
    PRC_HCG_NewLoop = 0,
    PRC_HCG_EndLoop = 1,
    PRC_HCG_IsoPlane = 2,
    PRC_HCG_IsoCylinder = 3,
    PRC_HCG_IsoTorus = 4,
    PRC_HCG_IsoSphere = 5,
    PRC_HCG_IsoCone = 6,
    PRC_HCG_IsoNurbs = 7,
    PRC_HCG_AnaPlane = 8,
    PRC_HCG_AnaCylinder = 9,
    PRC_HCG_AnaTorus = 10,
    PRC_HCG_AnaSphere = 11,
    PRC_HCG_AnaCone = 12,
    PRC_HCG_AnaNurbs = 13,
    PRC_HCG_AnaGenericFace = 14,
}
impl fmt::Display for PrcCompressedFaceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

#[repr(u32)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum PrcCompressedCurveType {
    PRC_HCG_Line = 0,
    PRC_HCG_Circle = 1,
    PRC_HCG_BSplineHermiteCurve = 2,
    PRC_HCG_Ellipse = 12,
    PRC_HCG_CompositeCurve = 13,
}
impl fmt::Display for PrcCompressedCurveType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

/// PRC_TYPE_TESS_Face.used_entities_flag
#[repr(u32)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum PrcTesselationFlags {
    PRC_FACETESSDATA_Polyface = 0x0001,
    PRC_FACETESSDATA_Triangle = 0x0002,
    PRC_FACETESSDATA_TriangleFan = 0x0004,
    PRC_FACETESSDATA_TriangleStrip = 0x0008,
    PRC_FACETESSDATA_PolyfaceOneNormal = 0x0010,
    PRC_FACETESSDATA_TriangleOneNormal = 0x0020,
    PRC_FACETESSDATA_TriangleFanOneNormal = 0x0040,
    PRC_FACETESSDATA_TriangleStripOneNormal = 0x0080,
    PRC_FACETESSDATA_PolyfaceTextured = 0x0100,
    PRC_FACETESSDATA_TriangleTextured = 0x0200,
    PRC_FACETESSDATA_TriangleFanTextured = 0x0400,
    PRC_FACETESSDATA_TriangleStripTextured = 0x0800,
    PRC_FACETESSDATA_PolyfaceOneNormalTextured = 0x1000,
    PRC_FACETESSDATA_TriangleOneNormalTextued = 0x2000,
    PRC_FACETESSDATA_TriangleFanOneNormalTextured = 0x4000,
    PRC_FACETESSDATA_TriangleStripeOneNormalTextured = 0x8000,
    PRC_FACETESSDATA_NORMAL_Single = 0x40000000,
}

#[bitfield]
#[repr(u32)]
#[derive(Debug)]
pub struct PrcTesselationBitField {
    pub PRC_FACETESSDATA_Polyface: bool,                     // 0x01
    pub PRC_FACETESSDATA_Triangle: bool,                     // 0x02
    pub PRC_FACETESSDATA_TriangleFan: bool,                  // 0x0004,
    pub PRC_FACETESSDATA_TriangleStrip: bool,                // 0x08
    pub PRC_FACETESSDATA_PolyfaceOneNormal: bool,            // 0x0010,
    pub PRC_FACETESSDATA_TriangleOneNormal: bool,            // 0x0020,
    pub PRC_FACETESSDATA_TriangleFanOneNormal: bool,         // 0x0040,
    pub PRC_FACETESSDATA_TriangleStripOneNormal: bool,       // 0x0080,
    pub PRC_FACETESSDATA_PolyfaceTextured: bool,             // 0x0100,
    pub PRC_FACETESSDATA_TriangleTextured: bool,             // 0x0200,
    pub PRC_FACETESSDATA_TriangleFanTextured: bool,          // 0x0400,
    pub PRC_FACETESSDATA_TriangleStripTextured: bool,        // 0x0800,
    pub PRC_FACETESSDATA_PolyfaceOneNormalTextured: bool,    // 0x1000,
    pub PRC_FACETESSDATA_TriangleOneNormalTextured: bool,    // 0x2000,
    pub PRC_FACETESSDATA_TriangleFanOneNormalTextured: bool, // 0x4000,
    pub PRC_FACETESSDATA_TriangleStripeOneNormalTextured: bool, // 0x8000,
    #[skip]
    unused: modular_bitfield::specifiers::B14,
    pub PRC_FACETESSDATA_NORMAL_Single: bool, // 0x40000000,
    #[skip]
    unused2: bool,                 // 0x80000000,
}
sa::const_assert_eq!(4, mem::size_of::<PrcTesselationBitField>());

/// PRC_TYPE_TESS_Face.sizes_wire
#[repr(u32)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum PrcFaceWireTessellationFlags {
    PRC_FACETESSDATA_WIRE_IsNotDrawn = 0x4000,
    PRC_FACETESSDATA_WIRE_IsClosing = 0x8000,
}

/// PRC_TYPE_TESS_3D_Wire.number_of_wire_indexes
#[repr(u32)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, TryFromPrimitive)]
pub enum Prc3DWireTessFlags {
    /// if the first point of this wire should be linked to the last point of the preceding wire
    PRC_3DWIRETESSDATA_IsClosing = 0x10000000,
    /// if the last point of this wire should be linked to the first point of this wire
    PRC_3DWIRETESSDATA_IsContinuous = 0x20000000,
}
