//! The live-preview frame: what a viewer is streamed while a model draws an
//! asset, relayed from the driver through the backend to the console.

use serde::{Deserialize, Serialize};

/// One re-rendered asset-generation frame, streamed live as the model draws.
///
/// This is what a viewer renders to watch the asset take shape: the `image` is
/// the frame's PNG, base64-encoded so it travels in the same JSON transport as
/// every other live update, and `frame`/`operationCount` let the UI show which
/// frame changed and how far along it is. A voxel run additionally carries the
/// frame's current [`mesh`](Self::mesh) — decoded from the `PartMesh`-shaped `.glb`
/// every voxel-family binary emits — so the live viewer can rebuild the part in 3D and
/// assemble the scene; a 2D sprite run leaves it `None`. A skinned run instead carries
/// its skin-preserving [`skinned_glb`](Self::skinned_glb) plus [`rig`](Self::rig) so the
/// viewer can deform it, and an audio run carries its clip [`audio`](Self::audio). It is
/// never persisted — the post-run view regenerates the asset from the recorded action
/// log instead.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetPreview {
    /// The frame this preview belongs to. A single sprite (or static voxel model) is
    /// always frame `0`; a sprite sheet uses the `draw-sheet --frame` index, and an
    /// animated voxel model uses the part's declared index.
    pub frame: u32,
    /// How many operations the frame's action log holds after this one — the
    /// frame's progress, shown alongside the image.
    pub operation_count: usize,
    /// The drawing operation that produced this frame (for example `fill_rect`),
    /// when reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    /// The frame's PNG, base64-encoded (no `data:` prefix; a viewer builds the
    /// data URL).
    pub image: String,
    /// The frame's current surface mesh, for a voxel run — decoded from the same
    /// `PartMesh`-shaped `.glb` the post-run viewer loads, so the live viewer can
    /// rebuild the part in 3D directly (it never re-meshes). `None` for a 2D sprite run
    /// (which streams only the PNG).
    ///
    /// The value is the voxel-mesh tool's `Mesh` as JSON (its `positions`, `normals`,
    /// `colors` and `indices` arrays). It is carried as a JSON value, like
    /// [`system`](Self::system) and [`rig`](Self::rig), because the type is the tool's
    /// and this crate depends on no tool: the driver decodes the `.glb` into it and
    /// the console reads the arrays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh: Option<serde_json::Value>,
    /// The frame's current authored `system.json`, for a particle run — so the live
    /// viewer can **simulate** the effect as it is authored, rather than show only the
    /// rendered still. `None` for every other kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<serde_json::Value>,
    /// The frame's current whole-body `.glb`, for a skinned run (`mc-skin`/`sn-skin`/
    /// `dc-skin`), base64-encoded (no `data:` prefix) — kept **raw** so its
    /// `JOINTS_0`/`WEIGHTS_0` and skin survive, letting the live viewer **deform** it
    /// by linear-blend skinning rather than show the undeformed rest mesh. Paired with
    /// [`rig`](Self::rig); `None` for every other kind (a plain voxel run decodes its
    /// glb into [`mesh`](Self::mesh) instead).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skinned_glb: Option<String>,
    /// The frame's current `rig.json`, for a skinned run — the bones/joints/animations
    /// the live viewer poses [`skinned_glb`](Self::skinned_glb) with. `None` for every
    /// other kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rig: Option<serde_json::Value>,
    /// The frame's current clip `.wav`, for an audio run, base64-encoded (no `data:`
    /// prefix; a viewer builds the data URL) — so a watcher can play the clip as it is
    /// built, the streamed PNG being the model's own waveform/spectrogram preview.
    /// `None` for every other kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
}
