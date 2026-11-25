# Stick v4.1.0 — Supporter link and export-folder shortcut (2026-11-23)

- The crown beside **Hazem** in the supporters dialog is now clickable and opens his Instagram profile.
- Successful exports now show a **folder button** beside **Done / تم** that opens the folder containing the exported file.
- The folder shortcut works for video/GIF, still images, Sprite Sheet, and Lottie JSON exports.

# Stick — Release notes / ملاحظات التحديث

> **Maintainer checklist:** Update the English and Egyptian Arabic notes for **every new release** before publishing. Never claim features that did not pass tests.

## English (US)

### Timeline menu, faster exports and color handling
- Widened and grouped the frame context menu; added an explicit Close action. Standard right-click menus also dismiss when clicking outside.
- Hazem's supporter total is now 12 EGP; the crown still opens his Instagram.
- Stream video frames directly into FFmpeg instead of writing temporary PNG files; reuse held frames and bound GIF/video rendering memory.
- Corrected premultiplied alpha handling for imported media and safe translucent marker previews. PNG keeps straight RGBA; modern video uses explicit BT.709 matrix/range metadata, and AVI uses BT.601 for legacy decoding compatibility.
- The editor uses cached group compositing for translucent layers within a bounded preview memory budget, matching the export layer-opacity model.


### Final wide-marker overlap fix
- Fixed the remaining dark triangle/star artifacts on wide translucent Marker curves.
- Marker bodies are now built as one concave outline and triangulated once, instead of overlapping translucent segment quads.
- Dense fitted marker paths are lightly distance-decimated before tessellation to keep very large marker sizes stable without changing the drawn shape.
- Endpoint caps remain outside the marker body, so transparency stays even from start to finish.


### Marker cleanup & faster pose rig controls
- Removed the legacy Drawing Assets library UI. **Drawing Assets / المكتبة** now stays focused on the Body Reference Studio.
- The Pose Rig popup now puts **Visible**, **Move joints**, and **Front/Back** together for fast switching while drawing.
- **Front** keeps the rig over the artwork; **Back** lets new and existing artwork render over the rig. The rig remains editor-only and is never exported.
- The translucent Marker now uses one shared-vertex strip instead of overlapping wide-line join triangles, removing the dark/black wedges that appeared at sampled points and bends.


### Pose rig, marker and brush import
- Click the skeleton icon to show the rig and drag its joints. Click again to hide it and resume drawing. Reopening it keeps the pose.
- The marker follows an open path. Separate strokes keep their own pen-up positions, including when a quick lift and press arrive in the same frame.
- Import TIFF, TGA, GIF (first frame) and Netpbm image tips, or open a ZIP containing ABR, MyPaint and image brushes. Individual image files keep their own names inside packs.
- Procreate imports read the shape mask. Krita presets with a missing tip now ask for the resource instead of loading the preview as a brush.

### Body Reference Studio & smooth raster curves
- **Drawing Assets** now opens directly into a dedicated **Body Reference Studio**. The previous palettes/reusable drawings/Auto Actions page is still available through **Old Library**.
- Added body-shape presets: **Classic, Slim, Standard, Athletic, Heavy, Tall, Short, Feminine and Cartoon**. Presets change real shoulder/hip/torso/arm/leg/head proportions rather than only renaming the same stick figure.
- Body proportions can be fine-tuned manually; the pose rig's two-bone IK uses the edited arm/leg lengths.
- Normal raster brushes now use bounded Hermite curve fitting and distance resampling at render time, keeping project points raster/editable while removing visible angular facets.
- Variable-pressure 2D Pen prefers one continuous anti-aliased pressure outline on normal curves instead of a chain of visible dabs; tight hairpins keep a safe fallback.
- Built-in textured raster brushes now place their stamps on the fitted centerline with denser per-brush spacing, reducing the comb/ribbed square-stamp look on curved strokes while preserving texture and pressure.

### Bugs fixed
- Tool cursor icons now use the same hotspot as the real click coordinate. Selection arrow tip, Lasso loop, Eyedropper nib and centered transform/vector/tool cursors no longer render down/right from the actual pointer.
- Added customizable canvas-view shortcuts: **V** mirrors the canvas left/right and **F** flips it top/bottom. Both are available in Settings → Shortcuts and the shortcut help window; Reset Shortcuts restores V/F.
- Existing installs that still had the retired V Transform compatibility alias are migrated to **M**, preventing V from triggering Transform and canvas mirror at the same time.
- Marker no longer paints a chain of visible rectangular texture blocks. It now renders as one continuous translucent constant-width marker stroke, matching the dedicated smooth-marker behavior used by mature painting apps.
- Marker round strokes are now composited as one filled coverage region, removing the darker half-circle endpoints and black overlap seams that could appear on bends.
- Square, triangle, pentagon and rectangular tips on normal raster brushes now form one continuous path instead of a row of repeated geometric blocks. Vector Ink/Vector Pen rendering is intentionally unchanged.
- Added a separate **Sketch Brush / فرشة الاسكتش** preset with a broader paper/bristle texture than Sketch Pencil while keeping deterministic preview/export rendering.
- Timeline **Size** now scales the frame cells, row height, ruler and layer-label area together from **60% to 180%**, with quick −/+ controls. Panel height remains independently adjustable.
- Watercolor, Airbrush, Sketch Pencil, Graphite Pencil, Dry Brush, Chalk, Charcoal, Pastel, Calligraphy and Soft Sketch now use denser per-preset dab spacing instead of one coarse universal spacing value.
- Grain-bearing pencil/chalk/dry brushes now smooth stamp travel direction over adjacent samples, reducing sudden square/zig-zag stamp rotation without removing their texture.
- Fast Brush and Vector Pen strokes now consume the real intra-frame pointer path instead of only the frame's final cursor position. Hover moves before the initial press are excluded, near-duplicate samples are dropped, and a 96-sample safety cap preserves the first/last points without allowing a stalled frame to enqueue unbounded work.
- Tablet pressure now latches per contact: once a real pressure sample proves the current contact is a pen, a missing/zero pressure reading during lift stays at zero instead of falling back to a full-pressure mouse dab. The next mouse contact resets normally to 100% pressure.
- Pointer-up catch-up is pressure-aware, preventing a synthetic thick endpoint/blob after a pressure-capable stylus leaves the surface.
- Added `RUST_DRAWING_APPS_REVIEW_2026_11_19.md`, a source-level review of Rnote, Graphite, JFPainter, rusty-painter, Umber, rustcreate, TraceDraw and KiPaint, including licenses, adopted ideas and larger renderer changes deliberately deferred for compatibility.
- Added one-click collapse/restore arrows for both the left Drawing Tools dock and the right Properties/Layers dock; collapsed docks leave a narrow edge handle so they can be brought back without opening Settings.
- Vector Nodes right-click editing is no longer gated by the left-mouse canvas capture. Right-click on a visible node removes it; right-click on a visible vector segment inserts a new editable node, with selected strokes preferred but never blocking another visible path.
- The normal **2D Pen** (formerly G-Pen label) now uses stronger corner-preserving smoothing, clean round pressure dabs for stylus width changes, and a safe line fallback on risky hairpins. Canvas preview and raster/video export share the same geometry.
- Ordinary round brushes no longer build a folded pressure polygon on a sharp reversal; risky hairpins fall back to a continuous line instead of producing wedges/blobs.
- Paint Bucket underpaint now accounts for Boundary Thickness and real stroke width, preventing the temporary solver mask from creating a white halo. Nested closed regions are inset slightly under their black outline so they stay independently paintable without a white ring.
- Vector Nodes cursor now uses the exact real pointer hotspot instead of the old generic down/right icon offset.
- Vector-node selection/right-click hit areas are larger and remain screen-size consistent while zooming.
- Right-click insertion now hit-tests the same smoothed vector curve that is actually rendered; right-clicking the inserted/selected node again removes it.
- Layer Effects now opens as a movable, closable floating window instead of a fixed popup menu.
- Added a true **Black & White** layer effect for strokes, text and media; canvas preview and export use the same grayscale conversion.
- Bucket fills now preserve immediate nested closed contours as holes. A circle inside another circle can be colored independently without the outer fill bleeding through the inner region.



### Quick color controls, customizable gestures and Online UI
- The radial color wheel now has a **Custom Color** center action. It opens an exact color picker instead of showing the old COLOR label.
- The quick color wheel now works with **Brush, Vector Pen and Bucket**. Its default keyboard shortcut is **Z**, and it can be rebound from Settings → Shortcuts.
- The hold-and-move **tool size** gesture and **opacity** gesture are now separately rebindable. Defaults remain **Alt** for size and **Ctrl** for opacity.
- Tool-size feedback now shows the active value in pixels plus a normalized percentage of that tool's supported size range; Sculpt reports its radius. Opacity still shows the selected-color preview dot and percentage.
- Removed the redundant right-panel **Split** slider. The draggable three-dot divider remains the single control for resizing Properties versus Layers.
- Online Create/Join screens now use icon cards and connection-method chips with delayed hover help. The UI distinguishes LAN, Tailscale and advanced Direct/Tunnel joins without changing Stick's working TCP/UDP room protocol.
- The online guide now documents provider-neutral public TCP endpoints. Stick does not bundle a tunnel client or claim third-party plans are permanently free.
- Added `ONLINE_ARCHITECTURE_REVIEW_2026-11-19.md` with the networking review and future transport recommendations. No project-schema or collaboration-protocol version change was required.

### Direct keyframe drag, sculpt/fill, radial menus and lip-sync audio
- Removed the temporary Copy +1 / Move +1 / Clear-selection quick icons from the timeline toolbar.
- In OLD, NEW and New mine, click a real keyframe once to select it, then hold the left mouse button and drag that selected keyframe; releasing drops it at the new frame. A gold drop preview follows the drag. New mine keeps Ctrl/Shift multi-select and Alt-drag copy.
- Vector Pen freehand input now applies bounded control-node smoothing before node reduction, reducing visible hand/tablet zig-zag while preserving deliberate hard corners.
- Sculpt/Finger now deforms vector strokes and Bucket/contour fills, preserves closed seams, rejects far-away strokes early, and avoids rebuilding already-dense geometry on every pointer event.
- Right-click on the Shapes tool canvas opens a circular six-shape chooser. Right-click with Brush or Vector Pen opens a circular color palette; choosing an item closes the menu.
- Holding Alt over the canvas still changes tool size, but now shows the live effective radius/size on the canvas. Holding Ctrl changes Brush/Vector/Shape opacity and shows a live color dot at the selected opacity.
- Floating menus/windows now restore the normal OS cursor and block drawing interactions behind them by checking the topmost egui layer; the feedback/developer window is part of the modal guard.
- Frame-by-frame audio scrub reuses one audio output, cancels the previous snippet, adds short pre-roll and plays 220–340 ms of context so spoken audio is clearer for lip-sync work.
- Audio import explicitly accepts WAV/WAVE in addition to the existing compressed formats.
- Added `PERFORMANCE_REVIEW_2026-11-19.md`; the hot-path review reduced sculpt allocations and eliminated thread-per-frame scrub previews without changing the project schema.

### The System, Transform and timeline interaction update
- Added **The System** as a seventh saved interface preset: deep navy HUD panels, cyan neon borders, gold accents, System-style canvas/timeline chrome, and a dedicated activation sound.
- The standalone normal **Selection** tool is removed from the visible toolbar. **Transform** now owns click selection, box selection, move, resize and rotate; the old Selection shortcut remains a compatibility alias to Transform.
- New-project Artist System intro is now **4 seconds**. Appearance and dismissal use different synthesized System UI sounds, and every appearance chooses one of exactly **100** rotating bilingual System words.
- Timeline quick actions for **Copy to next frame**, **Move keyframe forward** and **Clear selection** are icon-only controls shared by OLD, NEW and New mine. Their names appear as delayed tooltips after the configured two-second hover delay.
- Frame provenance is now preserved through next-key creation, clipboard paste and New mine multi-copy: inserted empty = green circle, copied pose = red circle, explicit blank = white circle with black outline, authored/drawn legacy content keeps its drawn-frame marker.

### Bugs fixed
- Level-up celebrations now last 4 seconds, can be disabled independently from General Settings, and Egyptian Arabic gains 100 new hand-written motivational/joke lines in addition to the existing message combinations.
- Vector Nodes right-click editing is now always active: right-click a node to remove it or a selected vector segment to insert a node. The old enable/disable checkbox and duplicate simplify action were removed; the tool now exposes a 10–100% Node Density control and uses a larger screen-consistent edit hit radius.
- Text objects now support persistent Free Transform rotation. Rotated text selection/hit-testing follows the visible text, and PNG/video export preserves the same text angle.
- Image/video properties now include opacity presets (100/75/50/25/10%) plus a continuous media-opacity slider.
- Layer rename is isolated to the layer name only; image/video media paths and keyframes are regression-tested to remain unchanged when a layer is renamed.
- Every layer now has a ✦ effects menu with non-destructive None/Warm/Cool/Rose/Mint/Night/Vintage presets. Effects are saved per layer and applied consistently to canvas media, strokes, fills, text, symbols and exports.
- The frame context menu now includes Cut Frame. Cut copies the visible pose to the frame clipboard and leaves a blank keyframe at the source; the redundant Clear Frame entry was removed because Blank Keyframe already performs the same content-clearing operation.
- Imported still images now get a dedicated timeline clip lane: drag the body to move the image exposure, drag the left grip to change its start, and drag the right grip to extend/shorten its end. The image end is stored explicitly so later timeline growth cannot accidentally extend it.
- Imported video clips remain fixed-duration but can be moved left/right as one unit. Image/video clip dragging and layer effects respect layer locks.

- Free Transform's custom cursor is now centered exactly on the real mouse hotspot. The previous generic tool-icon offset drew the visible transform cursor about 20 pixels down/right from the actual click/drag coordinate, making selection rectangles and handle interactions look misaligned.
- Transform selection, resize, rotate and X/Y axis hit-testing continue to use the same exact screen-space pointer coordinate, including after zoom, pan, canvas rotation or flips.

- Clip Timeline thumbnails now show the actual visible frame, including held exposures and evaluated tween in-betweens instead of appearing blank unless the cell itself contains an explicit keyframe.
- Clip thumbnails now preserve quick one-point pen dabs, contour fills, layer opacity, transformed Symbol artwork, positioned text markers and media footprints.
- Symbol instances are visible again in the normal canvas preview. Canvas, Onion Skin and Clip Timeline now honor the same symbol pivot/scale/rotation/translation semantics already used by raster/video export.

- Timeline/dock drags no longer leak into canvas tools: a Brush, Eraser or Sculpt gesture can start only when the primary-button press itself began on the canvas.
- Normal freehand raster brushes now use light corner-preserving centerline cleanup plus a speed-aware live stabilizer, reducing mouse/tablet micro-jitter while keeping deliberate sharp corners and catching up during fast strokes.
- Canvas preview and raster/video export share the same freehand smoothing path so ordinary line brushes do not change shape unexpectedly on export.

- Free Transform handle hit testing now uses the exact painted screen positions, fixing cursor/hitbox offsets after zoom, pan, rotation or canvas flips.
- Selection/marquee outlines are drawn from transformed corner points so visible bounds stay aligned with the canvas transform.
- Artist System and level-up overlays now block canvas/tool input behind the popup, preventing custom cursor and drawing interactions from leaking into background windows.

### What's new
- **External brush engine v2:** imported brushes are no longer forced through one generic stamp renderer. Krita auto brushes, Clip Studio round/auto tools, MyPaint brushes and Blender Grease Pencil drawing brushes use a dense continuous renderer; GBR/GIH/ABR and material-tip brushes keep bitmap stamp behavior.
- Existing imported brushes are now versioned and rebuilt automatically from the original source file Stick saved at import time, so parser fixes also repair brushes that were added in older Stick versions.
- Old KPP conversions that depended on a missing real GIH/GBR resource are disabled with a clear re-import message instead of continuing to draw a fake round brush.
- Continuous external brushes are oversampled at large sizes to prevent the visible circle/bead pattern shown by 100px imported brushes.
- Clip Studio `.sut/.sutg`, Krita `.kpp/.bundle/.myb/.gbr/.gih/.abr`, and Blender Grease Pencil `.blend/.asset.blend` continue through app-specific parsers and preserve the settings Stick can represent.
- **Updater TLS/CDN reliability fix:** update downloads now retry through fresh HTTPS connections when GitHub/CDN TLS streams fail, including errors such as `cannot decrypt peer's message`.
- If the Rust HTTPS transport still fails, Stick automatically falls back to Windows `curl.exe`/Schannel, then performs the same published-size, SHA-256, ZIP and PE validation before installing anything.
- **Krita KPP spacing/import fix:** Stick now keeps the real `brush_definition` spacing (for example 0.1 = 10%) instead of incorrectly replacing it with Krita's `SpacingValue` dynamics multiplier. This fixes imported brushes appearing as rows of separated dots.
- Disabled Krita dynamics are now respected: stale `ScatterValue`, rotation or opacity sensor data is ignored unless the corresponding pressure option is enabled.
- Predefined Krita brushes now load their real `.gbr` / `.gih` tip resources from beside the KPP, a sibling `brushes` folder, the local Krita resource folder, or from inside a `.bundle`. Multi-tip GIH data and brush-tip scale are preserved.
- If a KPP requires an external brush tip that is missing, Stick now refuses the fake round-tip fallback and reports the missing filename instead of silently creating the wrong brush.
- Launcher branding now shows **Stick + the app icon** instead of “Stick Studio”, and the mode prompt is now “What are you here to do?” / «انت داخل تعمل ايه ؟».
- The in-app updater now prefers a **compressed `Stick-update.zip`** from GitHub Releases, verifies its SHA-256, extracts and validates `Stick.exe`, then performs the existing safe restart swap. Legacy releases containing only `Stick.exe` still work.
- GitHub Actions keeps the normal `Stick-Windows` artifact small by not nesting the updater ZIP inside it; the compressed updater package is published separately to Releases.
- Added **Add brush / أضف فرشة** beside the built-in raster brush presets. Users can import multiple brushes, rename each one, optionally choose a custom icon, keep them permanently across restarts, and right-click an imported brush to delete it.
- Imported brush strokes use the existing project brush-texture ID and share the same canvas/export renderer. The Vector Ink/Vector Pen renderer is unchanged.
- Added brush import families for **Krita** (`.kpp`, `.bundle`, `.myb`, `.gbr`, `.gih`, `.abr`), **Clip Studio Paint** (`.sut`, `.sutg`, `.abr`, with legacy `.tos` compatibility fallback), and **Blender Grease Pencil Brush Assets** (`.blend` / `.asset.blend`), plus common image brush tips.
- Krita embedded tips/brush definitions and pressure sensors, Clip Studio size/flow/spacing/rotation fields and readable material images, and Blender opacity/flow/spacing/jitter/pressure flags are normalized into Stick's raster brush engine. Vendor-only dynamics without a Stick equivalent are explicitly marked as approximations.
- Brush presets now have deliberately different rendering personalities instead of sharing nearly identical round strokes: watercolor has mottled wet grain, marker uses a broad chisel footprint, airbrush has a soft spray falloff, sketch tools have paper tooth, and the existing dry/chalk/charcoal/pastel/calligraphy families keep their own procedural masks.
- Added a separate **Graphite Pencil / قلم رصاص** brush with broken graphite grain, lighter low-pressure opacity, darker high-pressure buildup, a narrower pencil footprint, and matching canvas/export rendering. The older Pencil preset is now labeled Sketch Pencil / سكتش so the two tools are clearly different.
- Timeline action markers are consistent in OLD, NEW and New mine: a newly inserted empty cel is a green circle, a copied/current-pose keyframe is a red circle, and an explicit blank keyframe is a white circle with a black border. The marker type is saved with the project and old files default safely to legacy visuals.
- Artist System has a cyan/gold cyber-HUD identity. Creating a new project shows the artist level, rank and XP progress for 4 seconds with animated electric border effects and System UI sounds.
- General Settings can disable the new-project Artist System intro; the global sound-effects switch still controls its sound.
- Free Transform uses a smaller, distinct transform-gizmo icon.

### Bugs fixed
- Smooth Ink now keeps a stable smoothed centerline for constant-pressure drawing and falls back safely on risky hairpin geometry instead of producing self-intersecting filled blobs.
- Tapered Pen keeps tapered ends but uses a safe pressure-dab fallback on sharp reversals that could fold the pressure outline over itself.
- Lasso does not expose or activate Free Transform X/Y axis handles; rotate/resize/axis gizmos belong only to Free Transform.

### Canvas-edge fill, independent tools, tablet and timeline fixes
- Paint Bucket now treats the physical canvas border as a valid fill boundary, so an otherwise-open contour can close against the edge of the paper without leaking.
- Drawing strokes and Onion Skin previews remain visible in the workspace outside the paper bounds; export bounds are unchanged.
- Brush, Vector Ink Pen, full eraser, partial eraser and shape outline sizes are isolated instead of sharing one size value. Vector Pen opacity/stabilizer are also independent from normal brushes.
- Hold **Alt** and move the pointer right/left or up/down to resize the active brush-like tool without changing other tools.
- Symmetry can show an optional divider and now previews the mirrored stroke live while drawing.
- Added a **Tablet Pen** settings tab with pressure enable/disable, sensitivity and minimum-pressure controls; pressure samples are filtered to reduce sparse-event width glitches.
- Interface presets are presented as **Main Interface** plus **Interface 1–6**.
- Timeline style 3 is labeled **New mine**. Added a Last Frame button beside First Frame, and removed the obsolete F5 Insert Frame action while retaining F4 Delete Frame and F6 Keyframe.
- Settings Profile now includes **Save current settings** beside Restore Defaults.
- **Manual QA still required:** real graphics-tablet drivers/backends vary; CI validates code paths but does not replace hardware testing.


### Interface style presets
- Added a saved Interface submenu to the hamburger menu.
- Presets: Current Stick, Cartoon, Serious, Cute Pink, Animate Style, Krita Style and Blender Style.
- Presets change Stick-owned colors, UI density, tool icon scale, timeline styling and dock proportions while preserving the same project format and commands.
- Named familiar styles are workflow-inspired only; Stick does not bundle third-party logos or assets or claim affiliation.


### Interface style presets
- Added a saved **Interface / الواجهة** submenu to the hamburger menu.
- Presets: Current Stick, Cartoon, Serious, Cute Pink, Animate Style, Krita Style and Blender Style.
- Presets change Stick-owned colors, UI density, tool icon scale, timeline styling and dock proportions while preserving the same project format and commands.
- Animate/Krita/Blender names describe familiar workflow-inspired layouts only; Stick does not copy third-party logos/assets or claim affiliation.


### Media, timeline and drawing interaction overhaul
- Round brush tips now get explicit circular start/end caps for ordinary G-Pen/freehand lines, not only vector/Fineliner strokes.
- Audio clips can be deleted with Delete or from a right-click menu and can be duplicated from the same menu/Media inspector.
- Multiple audio tracks are prepared paused and started together; scrub preview no longer stops after the first active Stream track.
- Imported video layers get a dedicated draggable timeline clip aligned to the frame ruler, with Duplicate/Delete context actions.
- Imported images/videos/audio automatically open a dedicated **Media** inspector. Canvas sizes moved into a separate **Sizes / المقاسات** tab beside Properties.
- Image layers can set a visible-through frame without flattening or deleting the source image.
- Media on the canvas supports right-click Duplicate/Delete.
- F4 is a visible Delete Frame/Time action beside F5, and F4 works directly in Animation mode.
- Keyframes can be dragged horizontally in OLD and NEW timelines too; Advanced retains multi-select and cross-layer moves.
- Bucket fills now honor horizontal/vertical symmetry seeds, not only drawn stroke symmetry.
- Vector node tool adds an optional right-click mode: remove an existing node or insert one on the nearest vector segment.
- Holding/dragging over side/top/bottom UI no longer pans the canvas; hand/space pan requires the pointer to actually be over the central canvas.


### Top bar navigation polish
- Added a visible **New project** command at the top of the hamburger menu.
- Starting a new project clears the active document/path/history safely and preserves the current Drawing/Animation workspace.
- If the current project has unsaved changes, Stick writes a recovery snapshot before creating the blank project.
- Removed **Credits / About program** from the hamburger menu.
- Added a separate **red heart** beside the yellow Support heart. The red heart opens the existing Stick links/credits window containing the YouTube and Instagram links.
- The yellow heart remains dedicated to optional program support.


### Round 2 video QA — brush, fill, workspace and online moderation
- Normal mouse G-Pen strokes no longer build a filled pressure polygon when pressure is effectively constant. This avoids self-intersecting black wedges on tight reversals and removes repeated full-outline triangulation from the common mouse drawing path; meaningful stylus pressure still uses the pressure-aware outline.
- Paint Bucket now references all visible non-guide layers by default while always creating the new fill on the active layer. The UI explains this cross-layer boundary behavior explicitly.
- Stick preferences are movable. Perspective mode now persists across launches and adds ray-density, horizon visibility and parallel-line angle controls.
- Added portable Settings Profiles with export/import and Reset Defaults. Profiles intentionally exclude recent project paths and Google account identity.
- The top toolbar is compact: Home, Undo, Redo, command menu and the yellow Support heart. Secondary import/export/update/settings/view actions live in the command menu.
- Symmetry H/V text labels were replaced with distinct drawn mirror icons.
- UI font fallback now includes Windows Arabic, emoji and symbol fonts when available, and RTL paired punctuation gets explicit mirroring after bidi reordering.
- Online hosts can right-click guest rows to rename nicknames, recolor tools or kick the guest; moderation roster updates remain host-authoritative.
- The right-side Canvas Size & Presets section is visible by default and keeps common YouTube, Instagram, TikTok/Reels and custom dimensions.
- Stick already uses the WGPU renderer; startup now requests the high-performance WGPU adapter unless the user explicitly overrides WGPU_POWER_PREF.
- **Validation gate:** Windows unit tests, Clippy and release build must pass before merge. Hardware stylus, two-PC online and long-session performance QA remain manual checks.


### Video QA improvements (experimental)
- Audio dragging now captures the pointer on press and computes movement from the original press position, so 1-frame slow drags do not depend on egui's drag threshold; it keeps one Undo snapshot per real clip move and avoids timeline drag-scrolling conflicts.
- The frame-number ruler supports holding the mouse button and moving across frames to scrub continuously.
- For unrotated paper, canvas drawing and onion ghosts are clipped to the paper bounds without changing project files or raster exports.
- Room hosts can copy password-free invitation links. Guests can paste a link to join a reachable LAN/private-network/public TCP host. Optional ZeroTier joins are explained alongside existing LAN/Tailscale.
- **Limitations:** invitations are not a global room relay or NAT bypass; an external host needs reachability. Stick's direct TCP traffic is not end-to-end encrypted. Real cross-network and stylus tests remain outstanding.
- Source: silent 9m47s screen recording and current code audit. Some brush/shape visual artifacts require separate controlled reproduction.

### Work Overhaul integration — experimental features
- Integrated five independently CI-tested change sets: adaptive vector smoothing, pen pressure de-jittering, canvas appearance/presets, Advanced Timeline keyframe move/copy, and five functional brush presets.
- Existing OLD/NEW timeline options and saved .stick projects remain supported; new work should be checked with a backup copy first.
- **Important:** the full multi-network Online/Relay redesign is **not** included in this integration. Host connections may still require LAN or an external accessible tunnel.
- **Validation:** the combined Windows CI test, Clippy and release build are mandatory before publication. Real stylus input, smoothness comparisons, cross-PC networking, and export-image visual checks remain **unverified**.

### Five real brush presets (experimental)
- Added functional Fineliner, Smooth Ink, Tapered Pen, Calligraphy and Soft Sketch Pencil presets with distinct pressure/width, stabilization or procedural nib behavior rather than placeholder names.
- All original presets remain available and existing project format and brush variant ordering are unchanged.
- **Pending:** Windows CI and hardware/visual QA. Arbitrary per-brush Hardness, Spacing and Pressure Curve controls are not yet shipped.

### Advanced Timeline keyframe editing foundation (experimental)
- An optional third timeline mode keeps the existing OLD/NEW views and audio lanes, with a distinct ruler and exposure visualization.
- Ctrl/Shift-click selects multiple authored keyframes. Drag moves them within/across unlocked tracks; Alt+drag duplicates them. Editing rejects collisions and out-of-bounds targets atomically, and successful operations use Undo.
- **Still pending:** Windows CI/manual GUI verification, audio trimming, multi-selection clipboard operations, exposure edge stretching, magnetic snapping and thumbnail caching. This release note is an under-review engineering change, not a shipped claim.

### Pressure stroke quality (experimental)
- Shared G-Pen outline now de-jitters small accidental movements on straight-ish strokes, resamples long movements by distance and limits hard-corner joins while keeping pressure variation.
- Renderer geometry is bounded, with no alteration of saved raw project stroke data and no change to the .stick schema.
- **Validation status:** unit regressions added; Windows CI and physical tablet/preview/export QA still required.

### Canvas appearance and preset sizes (experimental)
- General preferences: display-only border visibility/color/width, rounded stage corners, optional shadow, workspace background, live preview and defaults reset.
- Canvas Size Mode: explicit YouTube Thumbnail, HD and Full HD entries, aspect ratio locking and a confirmation when shrinking could hide existing artwork; preexisting sizes are retained.
- Local settings and project compatibility are preserved via serde defaults; exported images and videos do not include workspace chrome.
- **Validation pending:** Windows CI and hands-on GUI/rotation/export checks; not a published release.

### Vector geometry refinement (experimental)
- Adaptive vector interpolation samples long curved spans more densely while preserving intentional sharp corners, closed-loop seams and the exact editable control nodes.
- Square-cap extension is safer around duplicate nodes and disabled on closed vector contours. Added deterministic geometry regression tests.
- **Validation status:** awaiting GitHub Windows CI and hands-on preview/export comparison; not a shipped feature until merged and released.

### UI, canvas & brush update (experimental)
- **Timeline layers:** replaced ambiguous glyph boxes with drawn visibility-eye and open/closed lock icons. Right-click a layer title in either timeline layout to Rename, Duplicate, or Delete (with the existing confirmation and Undo).
- **Vector ink pen:** non-destructive smoothed rendering keeps editable control points. Choose round or square end caps, opacity, and stabilizer in tool properties; preview and PNG/video raster export use the same smoothed curve.
- **Canvas Size Mode:** right properties panel now has common social-media presets and custom width/height, limited to 16 megapixels for memory safety. Resizing changes the stage boundaries without stretching existing strokes and supports Undo/Redo.
- **Online and launcher guides:** added bilingual, closable manuals with visible buttons, explaining host rooms, security, Tailscale and the application's capabilities.
- **New texture brushes:** Charcoal and Soft Pastel use deterministic procedural patterns shared between live preview and export.
- **QA required:** successful Windows CI does not replace manual pen, UI and two-PC collaboration tests. The existing online protocol stays at v5. Projects from older builds remain supported; project files using new brush presets should be opened with this or newer Stick builds.

### Bugs fixed
- The eyedropper samples overlapping layers from the front and distinguishes a shape's outline from its fill. Partial erasing now removes a single dab when its edge touches the eraser.
- Opening, restoring or importing a whole document clears the previous document's undo history and active save path.
- **Google sign-in configuration:** official Windows builds now contain Stick Studio's public Google Desktop OAuth Client ID. Legacy empty/invalid saved IDs recover automatically, while valid developer overrides stay intact. Google audience is still in Testing; live browser sign-in and Drive restore require authorized testers and manual validation before broad rollout.
- **Support popup:** bounded window size and scrollable body, with accessible Close/X and Esc; prevents giant fullscreen-height popup caused by unconstrained RTL layout.
- **Google OAuth:** older settings with an empty client ID now pick up a public Desktop OAuth ID embedded in newer builds; Sign in is disabled with setup guidance until a valid client ID is present. Real Google sign-in still requires app-owner Google Cloud configuration and testing.
- **Online:** stroke packets now share the host edit-order gate with structural changes; local TCP integration test checks that bad room passwords are rejected. Internet access still requires LAN reachability or Tailscale, and real two-PC QA is outstanding.
- **Brushes:** removed forced minimum watercolor/airbrush halo alpha when actual stroke opacity is zero, with regression coverage.

### Artist workspace refresh
- **Art System / نظام الفنان:** an anime-inspired, bilingual artist profile with editable name, selectable local avatar and rank progress. XP award formulas and rank thresholds are no longer displayed in the settings UI.
- **Drawing:** improved lasso geometry and drag-to-move; free-rotation selection handle; canvas zoom anchored under mouse; right-drag canvas rotation with the Hand tool; paper, strokes and imported bitmap objects share the view transform.
- **Tools:** colored cursor previews, right-click quick choices for common tools, two extra geometric shapes (triangle/pentagon), and six persistent brush-tip shapes shared between live rendering and image/video exports.
- **Workspace:** user-controlled toolbar, properties dock, tool list and timeline visibility; enhanced layer rows; refreshed timeline controls and layout.
- **Text:** multiline text editor with joined-Arabic preview; local-font-based text rendering in exported images/video.
- **Persistence:** profile/settings saved under the Windows user profile, with read compatibility for prior executable-adjacent JSON files; new stroke tip field defaults to a round brush in older projects.
- **Known boundaries:** bucket fill operates on recognized closed stroke contours (now with improved near-closure tolerance), not arbitrary raster flood fill. Imported artwork/text still needs further parity tests for every transform. No third-party drawing software GPL code is copied into Stick.

### What's new
- **Editable imports:** imported images and video-frame layers can be selected and transformed on the canvas. Right-side properties offer position, size, rotation, horizontal/vertical flips, with matching raster export rendering. Video layer transforms follow its decoded frames.
- **Audio in timeline:** visible green waveform lanes below animation cels; drag audio strips to change start time. Right panel exposes mute, gain, clip timing and scrub mode. Timed clips mix during playback; very long audio waveform data is currently a limited preview.
- **Bucket options:** Precision, Dilate/Contract, and boundary Thickness adjust Stick's bounded contour solver, inspired by external animation software Grease Pencil behavior; these are not the same implementation as external animation software's fill.
- **Video memory safety:** the editor no longer caches every decoded movie frame on the GPU, and preview textures are bounded. Source files remain referenced by path; save copies of external media.

- DryBrush and Chalk add pressure-sensitive bristles and grain to the brush picker. Their patterns follow the saved stroke in the editor and raster exports.
- Room participants can see one another's named, colored pointers on the same frame. All peers need this protocol v5 build or a compatible newer build.
- Both timeline styles can duplicate a cel after the selection or insert one empty cel before it. Later keys on that layer shift forward and either edit supports Undo.
- **Official website groundwork:** added a responsive static Stick Studio homepage, Google API privacy policy, and terms of use under `docs/`. GitHub Pages activation and verified-domain production OAuth branding remain separate owner-controlled steps; no public site URL is claimed live yet.
- **Arabic support window fix:** donation copy now uses properly ordered, manually wrapped right-aligned Arabic lines, with corrected colloquial spelling and separated emojis to prevent Unicode bidi wrapping errors.
- **Download progress:** verified GitHub releases now show a real download progress bar, completed percent, and transferred/total megabytes before SHA-256 verification and restart.
- **Top-bar feedback:** the blue Feedback label now opens the same in-app Instagram / GitHub report dialog as the launcher.
- **Support modal:** improved its layout, included the developer's requested humorous optional-support message, and retained the original InstaPay link.
- **Online host moderation:** room host can kick members and change each member's cursor/tool color; roster colors relay to all clients.
- **Graceful host handoff (protocol v4):** the oldest admitted guest can take over the room and remaining clients reconnect when the current host leaves normally. Unexpected crashes, unreachable new hosts and full multi-PC QA remain unresolved; see STICK_ONLINE_GUIDE.md. This update is not a worldwide room directory.
- **Artist cloud profile continuity:** opt-in automatic profile-only Google Drive sync for XP, displayed artist name, progress and a bounded 256px avatar PNG; the local avatar path is never uploaded. Restoring a profile refreshes the visible avatar. Requires app-owner OAuth setup and actual Windows sign-in validation.
- **Settings discovery:** main top-right gear is larger, amber and high-contrast with a tooltip.
- **Online test stability:** Windows integration tests run serially to prevent hard-coded local TCP-port contention; this does not establish worldwide room discovery or internet-wide collaboration.
- **Google Sign in / cloud backups (requires developer OAuth setup):** added a new Sign in settings tab using browser-based Desktop OAuth with PKCE, Windows credential storage and private Google Drive appDataFolder for artist XP/level data and saved Stick project files. Supports manual backup, cloud listing, safe restore and opt-in automatic backup after local save. **The production release needs the app owner's Google OAuth Client ID configured first; see GOOGLE_DRIVE_SETUP.md.** External media files are not bundled yet.
- **Perspective guides:** start disabled on every application launch, independent of the prior stored guide setting. New 2-point perspective controls the left/right vanishing points plus horizon; guides remain non-destructive and excluded from exports.
- **Eyedropper accuracy:** visible eyedropper tip now aligns precisely with actual cursor hit point across icon scale levels and samples from the top visible layer first.
- **Simplified Online joining:** cleaner room cards, direct IP/password, and collapsible 8-digit code/help. The existing localhost TCP host↔guest handshake regression test covers the networking protocol; independent-network Tailscale access still depends on device sharing and firewall rules.
- **OLD / NEW Animation Timeline:** the Timeline gear now lets artists switch between the familiar **OLD Timeline** (unchanged original Stick grid) and **NEW Timeline**, a clearer exposure-sheet layout with colored keyframe diamonds, continuous held drawing strips, a time ruler, taller customizable layer rows and independent horizontal zoom.
- **Preserved data & workflows:** switching style only changes local UI preferences; frame/keyframe copy, paste, blank keys, deletion, playback, loop range and project file format keep working through the same underlying commands. Existing preferences load with OLD selected by default.
- **Feedback dialog:** clicking "Report a bug or suggest a feature" on the start screen now opens a closable window with the maintainer's Instagram @ablikash and the project's GitHub Issues page.
- **Source limitation:** the uploaded AniHady2D 0.1.1 archive contains a Windows installer and setup README, not timeline source code or a screenshot of its timeline. NEW uses exposure-sheet design ideas; it is not claimed to be a pixel-perfect copy of the old app. Other already available capabilities (audio/video export, layers, keyframes) were not duplicated.
- **Online privacy/diagnostics:** LAN/Tailscale discovery and room connections are now **explicit opt-in per session**, never started merely by opening the online lobby. The "Tailscale connected, 0 peers" case explains that the host must share device access or invite collaborators into the same tailnet. Room codes remain discovery-only; a remote friend can use the host 100.x IP if allowed.
- **Exit hang:** no synchronous audio-drain on application close. Unsaved documents get Save & Exit / Exit without saving / Cancel.
- **Brush stability:** G-Pen now draws a continuous pressure-aware outline as one translucent geometry rather than a stack of dark, overlapping circles. PNG/video export uses the same outline where possible. Freehand strokes no longer receive destructive fourfold Catmull-Rom oversampling at release.
- **Vector nodes:** vector pen no longer multiplies stored nodes eightfold; configurable node reduction and right-click advanced options, Shift selection and box selection of multiple nodes with combined dragging.
- **Sculpt / Finger:** new drag-to-deform tool with adjustable radius and strength, using local point sampling and smooth falloff. New external animation software-like X/Y translation arrows appear on selected content, alongside existing scale and rotate handles.
- **Cursor customization:** Personalization includes brush cursor size, tool icon size and optional circular halo (off by default).
- **Limitations:** Multi-node edit is an MVP, not full external animation software Bezier-handle or Grease Pencil parity. Two-computer remote connectivity depends on Tailscale permissions and still requires physical UI QA.  
- **Tools reliability sprint (all 13 toolbar tools reviewed):** Bucket now fills enclosed complex/concave freehand loops with proper tessellation, grows color beneath visible outlines to avoid white halos, supports small ink-edge clicks, and exposes adjustable gap closure and optional all-visible-layer reference. Redrawing the same region changes its color on top of older fills.
- **Erasers:** complete eraser detects long stroke segments even without nearby vertices; partial eraser clips lines at exact intersection points and both erasers sweep over fast pointer paths.
- **Selection & Eyedropper:** precise geometry hit tests replace overly broad bounding boxes; eyedropper picks actual fill/line colors from the frontmost visible layer.
- **Brush & Shapes:** stabilized strokes now retain the pointer-up endpoint; square drawing no longer collapses on perfectly vertical drags. Added comprehensive regression tests; see TOOLS_AUDIT_2026_11_17.md. Windows GUI/physical stylus QA still pending.
- **Friends online (optional Tailscale):** join a room hosted on your friend's own PC from a different Internet connection. Detect the local Tailscale 100.x address, list rooms from visible online tailnet peers using unicast discovery and copy a host address for direct joining. Requires the separately installed Tailscale client and permission to access the host; not a central public room service.
- **Free Transform:** 22px resize hit radii and 21px rotate hit radius, constant in screen coordinates across zoom; all four corners now resize from their opposite anchors. The handles are slightly easier to see.
- **Error guidance:** connection-refused diagnostics distinguish LAN-only addresses from remote Tailscale routes.
- **Online sync v3:** automatic host-relayed updates to strokes, erasing, bucket fill, layers, visibility, frame/keyframe edits, text and transforms. Creating a room explicitly makes your PC its TCP host/server. Existing LAN discovery, eight-digit codes and optional passwords remain.
- **Bucket fix:** fills a bounded region separated by internal lines (for example, either half of a bisected circle), rather than automatically filling the whole outer shape.
- **Timeline:** deleting frame 1 preserves artwork held on the old frame 2; text and symbol instance frames appear as occupied in the timeline.
- **Remaining limits:** overlapping simultaneous edits to the same array position can conflict, assets are not transferred, and two-PC Windows UI acceptance is still necessary. Use host resync if divergent.
- Bright two-tone vector icons for all 13 drawing tools and matching pointer icons; original short, musical game-style UI sound synthesis replaces one-note sine beeps.
- Paint Bucket: bounded raster mask + flood-region vector reconstruction for shapes enclosed by multiple freehand brush strokes; preserves prior direct vector contour filling. Complex cases still have limits.
- Online LAN preview: host the current project, find rooms on the local network, share a copyable 8-digit room code, enter code on LAN or direct IP, and see unique-color participants with names and self-reported levels in the right Layers dock. This is NOT an Internet-wide room directory or verified Google account system. Document-level edits now synchronize in protocol v3, subject to simultaneous-edit conflicts and real-PC QA.
- Perspective settings: non-destructive overlay square grid, parallel lines, or single-point perspective with adjustable spacing, color, line thickness, opacity and vanishing point.
- Persisted timeline panel height and adjustable dock separator (drag between Properties and Layers). Colors and Symmetry inspectors collapse to free space.
- Rollback list now shows the immediate previous release separately from TEN older releases without duplicating it.
- **Version Backup / النسخة الاحتياطية:** open Settings → Previous Versions to return to the preceding official release or choose from the ten latest eligible older GitHub builds, each showing its publication date. All downloads use the existing SHA-256-verified installer flow and require confirmation; unsaved projects block restart.
- **Layers dock refresh:** wider resizable third-party drawing software-inspired sidebar with live stroke thumbnails, quick layer-name search, colored selection, stroke/frame metadata, guide-layer context action, and existing visibility, lock, rename, duplicate, delete, move, opacity controls. No third-party drawing software GPL code imported.
- **Stick Online beta:** third option under Drawing and Animation. LAN-hosted rooms, optional password, participant limit, LAN discovery and manual direct IPv4 connection. Room-wide project edits, including layers and timeline, now flow through the host; Internet-wide discovery is not implemented. This preview is not end-to-end encrypted.
- Art System: anime-style bilingual artist profile with editable display name, local avatar image and a level/rank progress bar. Progress formulas are not displayed.
- Customizable workspace, cursor-anchored smooth zoom, transform rotation handle, draggable lasso selections, right-click tool choices and more versatile brush-tip geometry.
- A centered 10-second LEVEL UP panel shows one of 1,000 unique motivating lines in easy US English or Egyptian Arabic, with one of 100 small procedural sound variations.
- Settings gear: new Shortcuts, Colors & Timeline, and Personalization tabs with panel visibility controls; sound-effects toggle and artist profile.
- Improved right-side docks: separate scrolling Tool Properties and Layers panels, plus right-click Layer options to Rename, Copy, Paste, Duplicate and Delete.
- Updated timeline appearance, resize by dragging the upper border, and a Timeline gear for separate forward/backward Onion Skin count, tint and opacity.
- Use the original red heart on the left to open YouTube/Instagram, and the gold heart at the top center to open the optional InstaPay support dialog.
- Free Transform now supports dragging a resize handle, Vector Nodes can be picked and dragged, and drawing tools have distinctive cursor previews and sound cues.
- Bucket can also fill clearly closed freehand brush loops as well as closed vector shapes. Fills are included in image/video export.
- Right-click timeline frames to Copy, Paste, Clear, Delete, create Keyframes or Blank Keyframes; removing time shifts all unlocked layers together.
- Pick separate Onion Skin frame counts and transparency for earlier and later frames; adjust loop start/end and timeline zoom.
- Layers now have Delete (with confirmation), Duplicate, Move Up/Down, visibility and lock controls, and true 0–100% opacity.
- Shape tools now have size, opacity and optional solid fill. Brush and eraser cursors show their actual size. The Properties panel scrolls independently of the canvas.
- Closed vector shapes can now be filled by the Bucket tool; selection offers rectangular marquee and freehand lasso, and text can be dragged using the Selection tool.
- Public test builds now publish automatically on GitHub Releases whenever a new version on main passes Windows tests and build. All users can try the newest version.
- The Check for Updates button is available from the start screen and the drawing/animation toolbar. The installed build number is compiled into Stick.exe.
- Users can report a bug or suggest a feature directly from Stick using the GitHub Issues page and ready-to-fill templates.
- First-run What's New uses the real release notes embedded inside Stick.exe, shown in simple US English or Egyptian Arabic.
- Updates download in small chunks to reduce RAM use on low-end PCs.
- New Drawing and Animation tools: pressure-aware G-Pen sampling, better layer-opacity export, bilingual toolbar, and Onion Skin frame previews.
- Preview can skip fully offscreen stroke geometry on weaker PCs.
- Check for updates directly from the Stick toolbar. When nothing new is available, Stick tells you you're up to date.
- Updates are downloaded with a SHA-256 integrity check before they are installed.
- Stick restarts to finish a verified update, and shows What's New the first time you start the new version.

### Bugs fixed
- Vector Nodes tool no longer does nothing when clicked on a vector vertex.
- Selection has a visual transform handle to resize selected strokes/text in stage coordinates.
- Opening Tool Properties no longer forces the Layers list to scroll with it.
- Bucket previously ignored hand-drawn closed loops and lost their interior fill in the G-Pen renderer.
- Cosmetic XP keeps its progress after saving/reopening projects, without counting repeat strokes on the same frame.
- Fixed broken Arabic word order in Save Your Work Before Update notices.
- Fixed the Bucket painting a nearby line instead of finding a clicked enclosed vector shape.
- Fixed the layer delete control being missing; fixed opacity/visibility changes not always marking the document as unsaved.
- Removed an unsafe old Merge ↓ button that could silently drop keyframe content.
- Fixed the main problem: successful Windows builds on main were not being published as updates, so users saw no newer version.
- Fixed standalone Stick.exe reporting an old or missing version.txt instead of its own compiled build number.
- Fixed version checks that could offer the same build again.
- Fixed the updater getting stuck if GitHub can't be reached.
- Added an unsaved-work guard to avoid restarting with an unfinished project.

## العربية (المصري)

### تحسين قائمة الفريمات والتصدير والألوان
- قائمة الفريمات بقت أعرض ومقسمة حسب الوظيفة، وفيها زر إغلاق.
- تحديث تبرع حازم إلى 12 جنيه مع الحفاظ على رابط الإنستجرام.
- التصدير بيبعت الفريمات مباشرة لـ FFmpeg بدل ملفات PNG مؤقتة، وبيستفيد من الفريمات الثابتة مع تحديد استهلاك الذاكرة.
- تصحيح التعامل مع شفافية الماركر والصور المستوردة، وتحديد تحويل الألوان في الفيديو. PNG بيحتفظ بقيم RGB والشفافية.
- معاينة الطبقات شبه الشفافة بتستخدم نفس محرك تجميع الرسم الخاص بالتصدير، مع كاش محدود للذاكرة.



### إصلاح نهائي لتداخل الماركر العريض
- اتحلت مشكلة المثلثات/النجوم السودا اللي كانت لسه بتظهر على انحناءات الماركر الشفاف والعريض.
- جسم الماركر بقى بيتبني كحد خارجي واحد ويتم تقسيمه لمثلثات مرة واحدة، بدل قطع شفافة متداخلة فوق بعض.
- المسارات الكثيفة جدًا بتتخفف حسابيًا بشكل بسيط قبل الرندر عشان أحجام الماركر الكبيرة تفضل ثابتة من غير ما شكل الخط يتغير.
- نهايات الماركر فضلت خارج جسم الخط، فدرجة الشفافية تبقى متساوية من البداية للنهاية.


### تنظيف الماركر وتحكم أسرع في هيكل الوضعية
- اتشالت واجهة **مكتبة الرسم القديمة** نهائيًا، وزر المكتبة بقى مخصص لمرجع الجسم والوضعية.
- قائمة **هيكل الوضعية** بقى فيها **ظاهر** و**تحريك المفاصل** و**أمام/خلف** جنب بعض عشان تقدر تبدّل بسرعة أثناء الرسم.
- وضع **أمام** بيخلي الهيكل فوق الرسم؛ وضع **خلف** بيخلي الرسم يظهر فوق الهيكل. الهيكل للمساعدة فقط ومش بيدخل في التصدير.
- الماركر الشفاف بقى بيتبني كشريط واحد مشترك بدل مثلثات الـjoins المتراكبة، وده يمنع العلامات/المثلثات السودا اللي كانت بتظهر على نقاط المسار والانحناءات.


### الهيكل والماركر واستيراد الفرش
- اضغط أيقونة الهيكل عشان يظهر وتقدر تسحب مفاصله. الضغطة التانية بتخفيه وبترجّعك للرسم، ولما تفتحه تاني هتلاقي آخر وضعية زي ما سيبتها.
- الماركر بيتبع مسار القلم وبيسيب الفراغ بين الخطوط. وكل خط بينتهي عند مكان رفع القلم، حتى لو بدأت الخط اللي بعده بسرعة.
- تقدر تستورد صور سنون الفرش بصيغ TIFF وTGA وGIF (أول فريم) وNetpbm، وتفتح حزمة ZIP فيها فرش ABR أو MyPaint أو صور. كل صورة بتحتفظ باسمها جوّه الحزمة.
- استيراد Procreate بيقرأ شكل سن الفرشة. ولو فرشة Krita ناقصها ملف السن، البرنامج بيطلب منك تحدده.

### مرجع الجسم وتنعيم الفرش الراستر
- زر **المكتبة** بيفتح دلوقتي مباشرة نافذة **مرجع الجسم** بدل الزحمة القديمة. المكتبة القديمة لسه موجودة من زر **المكتبة القديمة** عشان تقدر ترجع للباليتات والرسومات والأوامر التلقائية.
- اتضافت أشكال جسم حقيقية: **كلاسيك، نحيف، عادي، رياضي، ضخم، طويل، قصير، أنثوي، وكرتوني**. كل اختيار بيغيّر نسب الكتفين والحوض والجذع والذراعين والرجلين والرأس فعلًا.
- تقدر تعدّل نسب الجسم يدويًا، والـIK بتاع اليدين والرجلين بيحسب الأطوال الجديدة بدل ما يفضل مربوط بجسم ثابت.
- الفرش الراستر العادية بقت تعمل **Curve Fitting + Resampling** للمسار وقت الرسم/العرض، فالكيرفات تبقى ناعمة زي إحساس الفيكتور لكن الخط يفضل Raster ومش بيتحوّل لفيكتور.
- قلم 2D مع ضغط القلم بيفضل يستخدم تغطية متصلة وناعمة في المنحنيات الطبيعية بدل سلسلة دابات تبان كمربعات/درجات، ومع اللفات الحادة جدًا فيه fallback آمن.
- الفرش ذات التكتشر بقت تحط الـstamps على نفس المنحنى المتنعم وبـSpacing أكثف لكل نوع، فشكل الشرائح/المشط اللي كان بيبان في الكيرفات يقل جدًا مع الحفاظ على خامة الفرشة والضغط.

### المشاكل اللي اتصلحت
- أيقونات الأدوات بقت مرتبطة بنفس **Hotspot الضغط الحقيقي**: سن سهم التحديد، مركز اللاسو، سن القطارة، ومركز أدوات التحويل/الفيكتور بقوا في نفس مكان الكليك بدل الإزاحة القديمة لتحت ويمين.
- اتضاف اختصارين قابلين للتخصيص للكانفا: **V** لعكس الكانفا يمين/شمال، و **F** لقلب الكانفا فوق/تحت. موجودين في الإعدادات → الاختصارات وكمان نافذة شرح الاختصارات، وReset Shortcuts بيرجعهم V/F.
- الأجهزة القديمة اللي كان عندها اختصار V القديم كـalias للتحويل بيتنقل تلقائيًا لـ **M** عشان V ماينفذش وظيفتين مع بعض.
- **الماركر** مبقاش سلسلة مربعات/ختمات مستطيلة ظاهرة؛ بقى Stroke واحد متصل شفاف وثابت العرض، أقرب لسلوك الماركر الحقيقي في برامج الرسم.
- الماركر الدائري بقى بيتكوّن كتغطية واحدة للـStroke كله، فاختفى نصف الدايرة الغامق عند الأطراف ومناطق السواد/التراكب اللي كانت بتبان مع الانحناءات.
- أشكال السن **مربع / مثلث / خماسي / مستطيل** في الفرش العادية بقت تعمل خط واحد متصل بدل صف مربعات أو مثلثات متكررة. **قلم الحبر/الفيكتور متساب زي ما هو من غير تغيير**.
- اتضافت فرشة مستقلة جديدة باسم **فرشة الاسكتش / Sketch Brush** بخامة ورق وشعيرات أعرض من Sketch Pencil، وبنفس النتيجة في المعاينة والتصدير.
- تحكم **حجم التايم لاين** بقى يكبّر ويصغّر الفريمات وارتفاع الصفوف والمسطرة ومساحة أسماء الطبقات مع بعض من **60% لحد 180%**، ومعاه زر − وزر +، وارتفاع لوحة التايم لاين لسه مستقل.
- المائية، البخاخ، السكتش، قلم الرصاص، الفرشة الجافة، الطباشير، الفحم، الباستيل، الخطاط والسكتش الناعم بقوا يستخدموا Spacing مخصوص وأكثف لكل نوع بدل قيمة واحدة خشنة لكل الفرش.
- فرش القلم/الطباشير/الجافة بقى عندها تنعيم لاتجاه الختمات بين العينات، فالتكتشر يفضل موجود لكن من غير دوران مفاجئ ومربعات متكسرة.
- الفرشة وقلم الفيكتور بقوا ياخدوا **كل حركات المؤشر اللي وصلت جوه نفس الفريم** بدل آخر مكان بس، فالحركة السريعة بالقلم أو الماوس تبقى أنعم ومايتفقدش منها نقط. حركات الـHover قبل بداية الضغطة بتترفض، والنقط شبه المكررة بتتشال، وفي حد أمان 96 عينة لكل فريم عشان أي تهنيج ما يعملش backlog ضخم.
- ضغط التابلت بقى مربوط بكل ضغطة لوحدها: أول ما البرنامج يتأكد إن الضغطة جاية من قلم بضغط حقيقي، لو قراءة الضغط اختفت أو وصلت صفر وقت رفع القلم مش هترجع فجأة لضغط ماوس 100% وتعمل بقعة في آخر الخط. الضغطة الجديدة بالماوس بترجع طبيعي لضغط كامل.
- إكمال نهاية الخط عند رفع المؤشر بقى واعي بالضغط، عشان مايتضافش طرف تخين صناعي بعد ما سن القلم يسيب السطح.
- اتضاف ملف `RUST_DRAWING_APPS_REVIEW_2026_11_19.md` بمراجعة فعلية لسورس Rnote وGraphite وJFPainter وrusty-painter وUmber وrustcreate وTraceDraw وKiPaint، مع التراخيص والأفكار اللي طبقناها والحاجات الكبيرة اللي اتأجلت عشان ما نكسرش توافق Stick.
- اتضاف سهم لإخفاء وإرجاع شريط أدوات الرسم على الشمال، وسهم تاني لشريط الخصائص والطبقات على اليمين. لما الشريط يستخبى بيفضل زر صغير على حافة البرنامج ترجعه منه فورًا من غير ما تدخل الإعدادات.
- كلك يمين في **نقاط الفيكتور** بقى مستقل تمامًا عن كلك الشمال: كلك يمين على النقطة يحذفها، وعلى أي جزء ظاهر من خط الفيكتور يضيف نقطة جديدة قابلة للتحكم، حتى لو كان فيه Stroke تاني متحدد قبلها.
- الفرشة الأساسية بقت **قلم عادي 2D** بسلوك أنعم قريب من قلم الرسم ثنائي الأبعاد: تنعيم محافظ على الزوايا، ضغط قلم دائري من غير تكتلات، ونفس الشكل في المعاينة والتصدير.
- الفرش الدائرية العادية بقت تتجنب الـpressure outline لو المسار عمل لفة حادة ممكن تكسره؛ بدل البقع والمثلثات السودا بيستخدم خط مستمر وآمن.
- الدلو بقى يحسب سُمك حدود التعبئة وسُمك الخط الحقيقي وهو بيدخل اللون تحت الخط، فرفع Boundary Thickness مش هيسيب هالة بيضا. والحلقات الداخلية بتدخل التعبئة تحت حدها الأسود مسافة صغيرة مع بقاء المساحة الداخلية مستقلة للتلوين.
- مؤشر أداة نقاط الفيكتور بقى فوق مكان الماوس الحقيقي من غير الإزاحة القديمة.
- كبّرنا Hitbox تحديد النقاط وكلك يمين، وبيفضل ثابت بصريًا مع الزوم.
- كلك يمين لإضافة نقطة بقى بيختبر نفس المنحنى الناعم الظاهر على الشاشة؛ وكلك يمين مرة تانية على النقطة المضافة/المحددة بيحذفها.
- مؤثرات اللير بقت نافذة مستقلة تقدر تحركها وتقفلها بدل القائمة الثابتة.
- اتضاف تأثير **أبيض وأسود** حقيقي للخطوط والنص والصور والفيديو، في المعاينة والتصدير.
- الدلو بقى يحافظ على الدوائر/الأشكال المقفولة اللي جوه بعضها كمساحات مستقلة؛ تعبئة الدائرة الخارجية مش هتلوّن اللي جوه الدائرة الداخلية.


### عجلة الألوان والاختصارات وواجهة الأونلاين
- نص **COLOR** اللي في نص عجلة الألوان اتبدل بزر **لون مخصص**؛ الضغط عليه بيفتح منتقي لون دقيق.
- عجلة الألوان السريعة بقت شغالة مع **الفرشة وقلم الفيكتور والبوكت**، واختصارها الافتراضي **Z** وقابل للتغيير من الإعدادات → الاختصارات.
- حركة الضغط المطول لتغيير **حجم الأداة** وحركة تغيير **الشفافية** بقوا اختصارات منفصلة قابلة للتخصيص. الافتراضي لسه **Alt** للحجم و**Ctrl** للشفافية.
- مؤشر الحجم بقى يعرض قيمة السُمك بالبكسل ومعاها نسبة مئوية من مدى حجم الأداة؛ أداة النحت بتعرض نطاق التأثير. مؤشر الشفافية لسه بيعرض نقطة بنفس اللون والنسبة الفعلية.
- اتشال شريط **تقسيم** الزيادة من فوق اللوحة اليمين؛ فاصل الثلاث نقط هو المتحكم الوحيد في مساحة الخصائص والطبقات.
- شاشات إنشاء الغرفة والانضمام اتعمل لها UI جديد بكروت وأيقونات وشرح Hover مؤخر. الطرق بقت واضحة: LAN، وTailscale، ودخول مباشر/نفق للمستخدم المتقدم، من غير تغيير بروتوكول الغرف الحالي.
- دليل الأونلاين بقى يشرح أي عنوان TCP عام قابل للوصول بدل ربط البرنامج بخدمة نفق واحدة، ومفيش ادعاء إن خطط الخدمات الخارجية هتفضل مجانية.
- اتضاف ملف `ONLINE_ARCHITECTURE_REVIEW_2026-11-19.md` لمراجعة بنية الشبكات والخطوات المستقبلية. صيغة المشاريع وبروتوكول التعاون الحالي ما اتغيروش.

### إيه الجديد؟
- **محرك الفرش الخارجية v2:** الفرش المستوردة مبقتش كلها بتترسم بنفس نظام الختمات. فرش Krita Auto وClip Studio العادية وMyPaint وفرش رسم Blender Grease Pencil بقت تستخدم مسار Continuous كثيف، بينما GBR/GIH/ABR والفرش اللي سنّها صورة تفضل Stamp زي طبيعتها.
- الفرش اللي اتضافت في نسخ Stick القديمة بقى ليها version وبتتعاد ترجمتها تلقائيًا من الملف الأصلي اللي Stick كان حافظ نسخة منه، يعني إصلاحات الـparser الجديدة هتصلح الفرش القديمة من غير ما تفضل محبوسة في التحويل القديم.
- لو KPP قديم محتاج GIH/GBR حقيقي والملف ناقص، الفرشة هتتعطل برسالة واضحة تطلب إعادة الاستيراد بدل ما تكمل ترسم دايرة مزيفة.
- الفرش الخارجية الـContinuous بقت تتعمل لها sampling أكثف مع الأحجام الكبيرة عشان تمنع شكل الدواير/الخرز اللي ظهر عند حجم 100.
- دعم عائلات Clip Studio `.sut/.sutg` وKrita `.kpp/.bundle/.myb/.gbr/.gih/.abr` وBlender Grease Pencil `.blend/.asset.blend` بقى مربوط بمسار تحويل خاص بكل عائلة بدل fallback واحد.
- **إصلاح موثوقية تحميل التحديثات:** لو اتصال GitHub/CDN رجع خطأ TLS زي `cannot decrypt peer's message`، Stick هيعيد المحاولة باتصالات HTTPS جديدة بدل ما يوقف التحديث فورًا.
- لو مكتبة Rust نفسها فضلت تفشل، البرنامج هيستخدم تلقائيًا `curl.exe` الخاص بويندوز/Schannel كمسار بديل، وبعدها يفضل فحص الحجم وSHA-256 والـZIP وملف EXE شغالين قبل التثبيت.
- **إصلاح استيراد فرش Krita KPP:** البرنامج بقى يحافظ على الـspacing الحقيقي الموجود في `brush_definition` (مثلاً 0.1 = 10%) بدل ما كان يستبدله غلط بـ`SpacingValue`، وده كان سبب إن الفرشة تطلع نقط/خرز منفصل.
- لو Scatter أو Rotation أو Opacity pressure مقفولين في Krita، Stick مش هيطبق قيم قديمة مخزنة جوه الـpreset من ورا المستخدم.
- فرش Krita اللي بتعتمد على سن خارجي `.gbr` أو `.gih` بقت تدور على السن الحقيقي جنب ملف KPP، أو في فولدر `brushes` المجاور، أو موارد Krita المحلية، أو جوه `.bundle`، مع الحفاظ على multi-tip والـscale.
- لو ملف KPP محتاج سن خارجي ومش موجود، Stick مش هيعمل دايرة عادية مزيفة مكانه؛ هيقول اسم الملف الناقص بوضوح عشان الفرشة متظهرش بشكل غلط.
- اتضاف خيار **أضف فرشة / Add brush** جنب الفرش العادية. تقدر تستورد أكتر من فرشة، تسمي كل واحدة، تختار لها صورة لو حابب، وتفضل محفوظة في البرنامج بعد القفل والفتح. وكلك يمين على أي فرشة مستوردة يظهر **حذف الفرشة**.
- الخطوط المرسومة بالفرش المستوردة بتستخدم نفس محرك المعاينة والتصدير، وبتتحفظ بمعرّف الفرشة داخل صيغة المشروع الحالية من غير تغيير الـschema. **قلم الحبر/الفيكتور متساب زي ما هو**.
- دعم استيراد عائلات فرش **Krita** (`.kpp` و`.bundle` و`.myb` و`.gbr` و`.gih` و`.abr`)، و**Clip Studio Paint** (`.sut` و`.sutg` و`.abr` مع fallback للصيغة القديمة `.tos`)، وBrush Assets الخاصة بـ**Blender Grease Pencil** (`.blend` و`.asset.blend`)، بالإضافة لصور سن الفرشة العادية.
- البرنامج بيحوّل الإعدادات اللي ليها مقابل فعلي في Stick زي السن والـspacing والـflow والشفافية والضغط والدوران والـjitter. أي Dynamic خاص بالبرنامج الأصلي ومفيش له مقابل في Stick بيتوضح إنه تقريب بدل ما البرنامج يدّعي تحويل 1:1.
- الفرش بقت مختلفة فعلًا في الشكل والملمس بدل ما تبقى خطوط شبه بعض: المائية فيها حبيبات وبلل غير منتظم، الماركر بسن عريض مشطوف، البخاخ بتدرج رذاذ ناعم، وفرش السكتش فيها ملمس ورق، مع الحفاظ على خامة الفرشة الجافة والطباشير والفحم والباستيل والخطاط.
- اتضافت فرشة جديدة مستقلة باسم **قلم رصاص** بتحاكي الجرافيت: حبيبات متقطعة، الضغط الخفيف أفتح والضغط القوي أغمق، سن أضيق، ونفس النتيجة في المعاينة والتصدير. وفرشة الرصاص القديمة بقى اسمها **سكتش** عشان يبقى الفرق واضح.
- علامات إنشاء الفريم بقت موحّدة في OLD وNEW وNew mine: الفريم الفاضي الجديد دائرة خضرا، والكي فريم اللي بينسخ الوضع الحالي دائرة حمرا، والكي فريم الفاضي دائرة بيضا بحد أسود. النوع بيتحفظ جوه المشروع، والمشاريع القديمة بتفضل متوافقة.
- نظام الفنان أخد شكل HUD جديد سماوي/ذهبي. مع كل مشروع جديد بتظهر نافذة المستوى والرتبة وتقدم XP لمدة 8 ثواني مع برق متحرك وصوت برق متولد جوه البرنامج.
- في الإعدادات العامة تقدر تقفل نافذة نظام الفنان الخاصة بالمشروع الجديد، وزر الأصوات العام بيتحكم في صوتها.
- أيقونة أداة التحويل بقت أصغر ومختلفة وواضحة عن أداة التحديد.

### المشاكل اللي اتصلحت
- رسالة ارتفاع المستوى بقت تظهر 4 ثواني، وتقدر تقفلها لوحدها من الإعدادات العامة، وضفنا 100 جملة وإفيه تحفيزي جديد بالمصري بجانب الرسائل الموجودة.
- أداة نقاط الفيكتور: الكلك اليمين بقى شغال تلقائيًا من غير Checkbox؛ على نقطة يحذفها، وعلى جزء من خط فيكتور محدد يضيف نقطة. شيلنا الخيارات المكررة وبقت الخصائص فيها شريط كثافة نقاط من 10% لـ100%، وكبّرنا مساحة الالتقاط بشكل ثابت مع الزوم.
- النص بقى يلف فعلًا بأداة Free Transform، والزاوية بتتحفظ في المشروع، والتحديد والـHit Test بيتبعوا النص الملفوف، والتصدير بيحافظ على نفس الزاوية.
- خصائص الصورة والفيديو فيها دلوقتي فلاتر شفافية سريعة 100/75/50/25/10% مع Slider حر.
- تغيير اسم اللير بقى يغيّر الاسم فقط؛ مسار الصورة/الفيديو والكي فريمات ما بيتلمسوش، ومعاه اختبار يمنع رجوع مشكلة اختفاء الميديا بعد إعادة التسمية.
- اتضافت علامة ✦ لكل لير للمؤثرات: بدون / دافئ / بارد / وردي / نعناعي / ليلي / فينتاج. المؤثر محفوظ على اللير فقط وبيظهر بنفس الشكل في الكانفا والتصدير على الصور والخطوط والتعبئة والنص والـSymbols.
- قائمة كلك يمين على الفريم بقت أرتب واتضاف «قص الفريم». القص يحط الوضع المرئي في الحافظة ويسيّب مكانه كي فريم فاضي؛ وشيلنا «امسح محتوى الفريم» من القائمة لأنه كان بيكرر وظيفة «كي فريم فاضي».
- الصور المستوردة بقى لها Clip lane مستقل: اسحب من المنتصف لتحريكها يمين/شمال، من الحافة الشمال لتغيير البداية، ومن الحافة اليمين لمد أو تقصير النهاية. نهاية الصورة بتتحفظ صراحة عشان ما تتمدش لو طول التايم لاين زاد بعدين.
- الفيديو يفضل بمدته الأصلية من غير Stretch، لكن تقدر تحركه يمين وشمال ككتلة واحدة. سحب الصورة/الفيديو والمؤثرات يحترموا قفل اللير.

- مؤشر أداة التحويل بقى في نفس مكان نقطة الماوس الحقيقية بالظبط. الإزاحة القديمة بتاعة أيقونات الأدوات كانت بتخلّي شكل مؤشر Transform ظاهر حوالي 20 بكسل يمين وتحت مكان الضغط، وده كان بيخلي مربع التحديد والـHitbox يبانوا بعيد عن الماوس.
- التحديد والتكبير/التصغير والتدوير ومحاور X/Y في Transform بيستخدموا نفس إحداثيات الشاشة الدقيقة حتى مع الزوم والتحريك وتدوير أو عكس الكانفا.

- Clip Timeline بقى يعرض المحتوى المرئي الحقيقي للفريم، بما فيه الـHold/Exposure والفريمات البينية بتاعة الـTween، بدل ما الخلية تبان فاضية لو مفيش Keyframe صريح على نفس الرقم.
- صور الفريمات المصغرة بقت تعرض ضغطة القلم المفردة، الـFill، شفافية اللير، رسومات الـSymbol بعد التحويل، ومكان النص والوسائط بشكل أوضح.
- الـSymbol instances رجعت تظهر في معاينة الكانفا العادية، والكانفا والـOnion Skin والـClip Timeline بقوا يستخدموا نفس Pivot/Scale/Rotation/Translation الموجود في التصدير.

- سحب التايم لاين أو أي Dock مبقاش يتسرّب لأدوات الكانفا: الفرشة أو الممحاة أو النحت مش بيبدأوا حركة إلا لو ضغطة الماوس الأساسية نفسها بدأت جوه الكانفا.
- الفرش الحرة العادية بقت تستخدم تنعيم خفيف يحافظ على الزوايا مع Stabilizer متكيّف مع سرعة الحركة، عشان يقلّل رعشة الماوس/القلم من غير ما يعمل تأخير مطاطي في السحبات السريعة.
- معاينة الكانفا والتصدير Raster/Video بقوا يستخدموا نفس مسار تنعيم الفرش الحرة، عشان شكل الخط مايتغيّرش بشكل مفاجئ وقت التصدير.

- الحبر الناعم بقى يرسم بخط مركزي ناعم وثابت مع الضغط الثابت، وبيستخدم مسار آمن لو الحركة فيها رجوع حاد بدل ما ينتج شكل أسود متداخل.
- القلم المدبب حافظ على بداية ونهاية مدببة، ومع الانعطافات الخطرة بيستخدم رسم ضغط آمن بدل Polygon ممكن يلف على نفسه.
- أداة التحديد والتحديد الحر مبقوش يعرضوا أو يشغلوا مقابض محور X/Y الخاصة بالتحويل؛ التدوير والتكبير والمحاور بقوا لأداة التحويل بس.

### إصلاح حدود الدلو وفصل الأدوات وقلم التابلت والتايم لاين
- الدلو بقى يعتبر حدود الكانفا نفسها حاجز للتعبئة، فلو الرسمة مفتوحة من ناحية طرف الورقة والطرف هو اللي بيقفلها، اللون يفضل جوه المنطقة.
- الرسم والأونين سكين يقدروا يظهروا برا حدود الورقة داخل مساحة العمل، من غير تغيير حدود التصدير.
- حجم الفرشة، قلم الحبر/الفيكتور، الممحاة الكاملة، الممحاة الجزئية وسُمك الأشكال بقوا منفصلين عن بعض. شفافية ومثبت قلم الفيكتور بقوا منفصلين عن الفرشة العادية كمان.
- تقدر تضغط مطول على **Alt** وتحرك الماوس يمين/شمال أو فوق/تحت لتغيير حجم الأداة النشطة فقط.
- الرسم المتماثل فيه اختيار لإظهار خط فاصل، والجهة المقابلة بتظهر مباشرة أثناء الرسم بدل ما تستنى لحد ما ترفع القلم.
- اتضافت خانة **إعدادات القلم** للجرافيك تابلت: تشغيل/قفل حساسية الضغط، قوة الحساسية، وأقل ضغط. واتضاف فلتر لعينات الضغط لتقليل الجلتشات الناتجة عن العينات المتقطعة.
- الواجهات بقت تظهر باسم **الواجهة الرئيسية** ثم **واجهة 1–6**.
- اسم التايم لاين الثالث بقى **New mine**، واتضاف زر آخر فريم جنب أول فريم، واتشال F5 القديم مع الاحتفاظ بـ F4 للحذف وF6 للكي فريم.
- ملف الإعدادات بقى فيه زر **حفظ الإعدادات الحالية** جنب استعادة الإعدادات الافتراضية.
- **لسه مطلوب اختبار عملي:** اختلاف تعريفات وأجهزة الجرافيك تابلت محتاج تجربة فعلية على جهاز أحد المختبرين؛ نجاح CI مش بديل لاختبار الهاردوير.


### تحسينات فيديو الجلتشات (تجريبية)
- تم ضبط سحب الصوت من أول ضغطة وحفظ Undo واحد عند تحريكه، ومنع سحب مساحة التايم لاين من مقاطعة سحب المسار.
- الضغط والسحب فوق أرقام الفريمات يتيح التنقل المتصل، والرسومات خارج الورقة غير المدارة تتقص بصريًا في المعاينة.
- روابط دعوة جديدة من الهوست لا تتضمن كلمات سر؛ وZeroTier كاختيار إضافي مع استمرار LAN وTailscale.
- **ملاحظة:** الدعوة ليست سيرفر عالميًا ولا تتجاوز حجب الراوتر. اتصال TCP المباشر غير مشفّر طرف لطرف، وتجربة جهازين ما زالت مطلوبة.

### تحديث Work المجمع — خصائص تجريبية
- تم تجميع خمس حزم: تحسين نعومة الفيكتور، وتنعيم مسارات الفرش مع الضغط، وتخصيص حدود الكانفا والمقاسات، وتايم لاين متقدم لتحريك ونسخ الكي فريمات، وخمس فرش جديدة.
- التصميم القديم للتايم لاين وملفات المشاريع القديمة ما زال مدعومًا، لكن يُفضّل اختبار نسخة احتياطية من أي مشروع مهم.
- **الأونلاين بين شبكتين مختلفتين لم يتم إصلاحه في هذه الدفعة**، وقد يظل محتاج LAN أو وسيلة اتصال خارجي.
- **التحقق:** مطلوب نجاح اختبارات Rust وClippy وبناء ويندوز للنسخة المجمعة قبل النشر؛ اختبار التابلت وتجارب الواجهة والتصدير والأونلاين الواقعية لم تتم بعد.

- **تحديث واجهة وستيك للرسم:** أيقونات واضحة للعين والقفل في التايم لاين، وقائمة كليك يمين لإعادة تسمية اللير والدبلكيت والحذف، وتحسين الفيكتور بمنحنيات أنعم وأطراف مربعة أو دائرية، ومقاسات جاهزة ومخصصة للكانفا، ودليل أونلاين ومميزات البرنامج بالعربي والإنجليزي مع إغلاق سهل، وفرشتي فحم وباستيل جديدتين. لازم نجرب الأداء والقلم والأونلاين على جهازين فعليين رغم نجاح الاختبارات الآلية. تغيير مقاس الكانفا لا يمط الرسومات القديمة.
- **تجهيز موقع Stick Studio:** ضفنا صفحات رسمية للبرنامج وسياسة الخصوصية وشروط الاستخدام داخل `docs/`. تفعيل GitHub Pages وتوثيق الدومين للنشر العام على Google خطوات منفصلة، والموقع مش بنعتبره منشور قبل التأكد.
- **تجهيز تسجيل Google:** أضفنا معرّف OAuth العام الخاص بـStick Studio في إصدارات ويندوز، مع إصلاح الإعدادات القديمة الفارغة أو غير الصحيحة. ما زال Google في وضع Testing، ولا نعتبر تسجيل الدخول والنسخ السحابي مختبرين عمليًا قبل تجارب أجهزة ويندوز.

### المشاكل اللي اتصلحت
- **الصور والفيديو:** بقى ممكن تحديد الصور وفريمات الفيديو، تحريكها وتدويرها وتغيير مقاسها وقلبها من أدوات التحديد والخصائص، والتحويل يظهر كمان في التصدير.
- **الصوت:** مسارات صوت بموجات واضحة تحت الفريمات، والسحب لتغيير البداية، والتحكم في التوقيت ومستوى الصوت والكتم من خصائص اليمين.
- **الدلو:** Precision وDilate/Contract وThickness بضوابط آمنة على خوارزمية التعبئة الخاصة بـStick؛ مستوحاة من برامج أنيميشن أخرى وليست نفس الكود.
- **الأداء:** تخفيف استهلاك ذاكرة كارت الشاشة بعدم تخزين كل فريمات الفيديو المفتوحة كمجموعة خامات دائمة. ملفات الوسائط الخارجية لازم تفضل موجودة.

- القطّارة بتختار من الطبقة الظاهرة في الأمام وبتفرّق بين لون حدود الشكل وتعبئته. الممحاة الجزئية بتمسح نقطة الفرشة لما تلمس حافتها.
- فتح مشروع أو استرجاع نسخة أو استيراد مشروع كامل بيمسح سجل التراجع ومسار الحفظ بتوع المشروع السابق.
- **نافذة الدعم:** حجم ثابت ونص قابل للتمرير وزر إغلاق ظاهر واختصار Esc، لحل مشكلة النافذة الممتدة خارج الشاشة.
- **تسجيل جوجل:** إصلاح قراءة Client ID من النسخة الجديدة عند وجود إعداد فارغ محفوظ من نسخة قديمة، مع منع فتح تسجيل دخول غير مُعد وإظهار التعليمات. الدخول الفعلي يحتاج إعداد OAuth من المطوّر واختبار حساب حقيقي.
- **الأونلاين:** توحيد ترتيب إرسال الرسم والتعديلات واختبار رفض كلمة السر الخطأ. الاتصال خارج الشبكة المحلية ما زال يحتاج Tailscale أو عنوانًا متاحًا، واختبار جهازين مطلوب.
- **الفرش:** تصحيح ظهور أثر لوني للـWatercolor والـAirbrush عند الشفافية صفر.

### تطوير مساحة الفنان
- **نظام الفنان:** بروفايل بشكل أنمي واسم قابل للتغيير وصورة من الجهاز وشريط تقدم، من غير عرض معادلات احتساب النقاط.
- **التحديد والتحويل:** اللّاسو بقى يحسب الخطوط المتقاطعة ويتحرك بعد التحديد، وضفنا مقبض لتدوير التحديد بحرية.
- **الكانفا:** زووم ناحية مؤشر الماوس، تدوير اللوحة بالكليك اليمين مع أداة اليد، وتدوير الورقة نفسها مع الرسومات والصور المستوردة.
- **الأدوات:** مؤشرات بألوان مميزة، خيارات بالكليك اليمين، مثلث وخماسي، وست أشكال لرأس الفرشة محفوظة مع المشروع وموحدة بين المعاينة والتصدير.
- **الواجهة:** إخفاء وإظهار الجنبين والشريط العلوي والتايم لاين من التخصيص، وتجديد صفوف الطبقات وشكل الخط الزمني.
- **النصوص:** محرر نص متعدد الأسطر مع معاينة عربية متصلة، ورسم النصوص في التصدير باستخدام خط محلي من Windows.
- **التوافق:** حفظ الإعدادات والبروفايل في مجلد المستخدم القابل للكتابة مع استيراد الملفات القديمة، وفرش المشاريع القديمة تفضل دائرية تلقائيًا.
- **القيود الحالية:** الدلو بيلون المسارات المقفولة اللي بيتعرف عليها، ولسه مش Flood Fill كامل على أي سطح Raster. لم يتم نسخ أكواد third-party drawing software الخاضعة لترخيص GPL.

### إيه الجديد؟
- فرشتا DryBrush وChalk بتضيفا أثر شعيرات وحبيبات بيتغيّر مع ضغط القلم، وبيظهر بنفس النمط بعد الحفظ وفي تصدير الصور.
- في الغرفة هتشوف مؤشر كل رسّام باسمه ولونه على نفس الفريم. عشان تتصلوا مع بعض استخدموا نسخ متوافقة مع بروتوكول الغرف v5.
- في التايم لاين القديم والجديد تقدر تكرّر الخلية بعدها أو تدخل خلية فاضية قبلها. المفاتيح التالية على نفس الطبقة بتتحرك، وتقدر تتراجع عن العمليتين.
- **تصحيح نافذة الدعم:** ضبط صياغة رسالة الدعم المصري وتقسيمها إلى سطور قصيرة بمحاذاة اليمين لمنع تلخبط ترتيب الحروف والكلمات أثناء العرض.
- **شريط التحميل:** النسبة المئوية وحجم الملف المحمّل أثناء تحديث Stick، ثم التحقق من سلامة الملف قبل التثبيت.
- **الإبلاغ:** زر «ابلغ عن مشكلة» فوق بيفتح نفس نافذة GitHub وInstagram الموجودة في الشاشة الرئيسية.
- **الدعم:** شكل نافذة القلب الأصفر اتنظّم ونص الدعم اتغيّر حسب طلب المطوّر، مع الحفاظ على رابط InstaPay.
- **إدارة الهوست:** طرد أعضاء الغرفة وتغيير لونهم ولون مؤشر أدواتهم من قائمة إدارة الغرفة، مع مزامنة الألوان.
- **نقل الهوست عند المغادرة الطبيعية:** عند خروج صاحب الروم، أول ضيف دخل في جلسة الهوست الحالية يقدر يستلم السيرفر، وباقي الأعضاء يعيدوا الاتصال تلقائيًا عبر LAN أو Tailscale. البروتوكول بقى v4. ده مش حل لانقطاع الكهرباء المفاجئ أو غياب الاتصال بالهوست الجديد، ولسه محتاج اختبار على أكثر من جهاز.
- **الحفظ السحابي لملف الفنان:** المزامنة الاختيارية بقت ترفع مستوى الفنان والاسم والتقدم والصورة الشخصية المصغرة بشكل مستقل عن حفظ المشروع. الصورة بتتخزن كصورة صغيرة مش كمسار من الجهاز، وتظهر عند الاستعادة. لازم إعداد OAuth واختبار جوجل فعليًا.
- **ترس الإعدادات:** أكبر ولونه ذهبي واضح في أعلى اليمين، مع شرح عند الوقوف عليه.
- **اختبارات الأونلاين:** تشغيل اختبارات الاتصال بالتتابع لتجنب تعارض المنفذ المحلي. كود الـ8 أرقام ما زال مش خدمة غرف عالمية.
- **تسجيل الدخول وجوجل درايف:** تبويب تسجيل دخول جديد، باستخدام متصفح جوجل الرسمي وWindows Credential Manager. تقدر تحفظ مستوى الرسام ومشاريعه المحفوظة يدويًا أو تفعل النسخ التلقائي بعد حفظ المشروع، وتسترجع نسخة بدون الكتابة فوق المشروع المحلي. **تشغيل جوجل لكل المستخدمين محتاج إعداد Google OAuth Client ID للتطبيق في Google Cloud أولًا؛ التفاصيل في GOOGLE_DRIVE_SETUP.md.** الملفات الخارجية للصوت/الصور لسه مش جزء من النسخ السحابية.
- **المنظور:** يبدأ دائمًا مقفول عند تشغيل Stick، ومعاه منظور جديد بنقطتي تلاشي تقدر تتحكم في الأولى والثانية وفي ارتفاع الأفق، من غير ما خطوط الإرشاد تظهر في التصدير.
- **القطارة:** رأس أيقونة القطارة بقى على مكان النقر الفعلي بدل الإزاحة القديمة، واللون بيتاخد من أعلى طبقة ظاهرة.
- **الانضمام أونلاين:** غرف بشكل أوضح، دخول مباشر بعنوان IP/كلمة السر، وكود الغرفة وتعليمات الشبكة داخل قسم اختياري. الاختبار الموجود للهوست والضيف عبر TCP المحلي بيشتغل؛ الاتصال عبر الإنترنت الحقيقي لسه يحتاج ضبط Tailscale وصلاحيات الشبكة.
- **OLD / NEW Timeline:** من ترس التايم لاين تقدر تختار الشكل القديم OLD زي ما هو، أو NEW بتصميم صفوف أوضح، شريط استمرار الرسم بين الفريمات، علامات كي فريم، ومسطرة توقيت. لكل شكل تكبيره الخاص، وطول صف اللير قابل للتعديل.
- **أمان المشاريع:** تغيير الشكل بيغيّر إعدادات العرض فقط، مش البيانات ولا ملفات المشروع. عمليات النسخ والحذف والكي فريم والتشغيل والتكرار شغالة بنفس المحرك.
- **التواصل:** زر «بلّغ عن مشكلة أو اقترح ميزة» في الواجهة الرئيسية بيفتح نافذة قابلة للإغلاق فيها إنستجرام @ablikash و GitHub Issues.
- **ملاحظة مرجعية:** ملف AniHady2D اللي اترفع فيه Setup.exe و README فقط، مش كود أو صورة للتايم لاين. NEW مستوحى من فكرة التايم لاين القديم، مش نسخ مطابق مؤكد.
- **خصوصية الأونلاين:** مفيش بحث عن غرف أو اتصال غير لما المستخدم يختار تشغيل الأونلاين بنفسه. لو Tailscale متصل لكن 0 جهاز ظاهر، البرنامج بيوضح خطوة مشاركة جهاز الهوست أو دعوة أصحابك للشبكة.
- **إصلاح الخروج:** إلغاء انتظار مؤثر الصوت عند قفل البرنامج، مع خيارات احفظ واخرج / خروج بدون حفظ / إلغاء.
- **فرشة G-Pen:** الضربة بقت خط متصل بحساب ضغط القلم بدل دوائر شفافة بتتراكب وتتسبب في البقع السودا. والتصدير يستخدم نفس شكل الضربة. وقفنا التنعيم اللي كان بيخلق نقط زيادة وبيشوّه بعض الزوايا.
- **الفيكتور:** عدد نقاط أقل في الرسم الجديد، تحكم في نسبة تقليلها من كلك يمين، وتحديد Shift أو مربع تحديد لنقل نقاط متعددة.
- **أداة النحت / الصباع:** حركة مرنة لجزء من الخط بالحجم والقوة اللي تختارهم؛ وأسهم X/Y جديدة لتحريك العنصر المحدد أفقيًا أو رأسيًا زي واجهات 3D.
- **التخصيص:** تكبير/تصغير مؤشر الفرشة وأيقونة الأداة بجوار الماوس، وإمكانية إخفاء أو إظهار الدائرة المحيطة (مقفولة افتراضيًا).
- **ملاحظة:** ما زال مطلوب اختبار عملي على جهازين Windows، والفيكتور مش نسخة كاملة من Bézier/Grease Pencil في external animation software.
- **تحديث تطوير أدوات الرسم:** راجعنا الأدوات الـ13؛ أصلحنا الدلو مع الحلقات والخطوط المتقاطعة والأشكال المقعّرة، وخفّفنا الفراغات البيضا بين التلوين وحدود الخط. اتضاف قفل فجوات قابل للتعديل وخيار استخدام حدود كل الليرات الظاهرة.
- **الممحاة الكاملة والجزئية:** بقوا بيحسبوا تقاطع الممحاة مع الخط نفسه وبيمسحوا أثناء الحركة السريعة، حتى لو مفيش نقاط خط تحت المؤشر بالضبط.
- **التحديد والقطارة:** الضغط على الخطوط بقى أدق، والقطارة بتجيب ألوان الخط أو التعبئة من أعلى لاير ظاهر.
- **الفرشة والأشكال:** الاحتفاظ بنهاية حركة الفرشة، ومنع المربعات الفاضية لو سحبت بشكل عمودي تمامًا. اختبارات الآلية اتوسعت، ولسه مطلوب تجربة الواجهة الحقيقية.
- **اللعب مع أصحابك على نت مختلف:** دعم Tailscale اختياري من غير بورت فوروارد. برنامج Stick بيكتشف عنوان 100.x الخاص بجهازك والغرف اللي يقدر يوصل لها على شبكة Tailscale، وبيسمح بنسخ عنوان الهوست للانضمام المباشر. يلزم تنزيل Tailscale وتسجيل الدخول والسماح للأصحاب بالوصول للجهاز؛ مش سيرفر غرف عالمي.
- **أداة التحويل:** تكبير مساحة الضغط غير المرئية للمقابض وتثبيتها مع الزوم، مع تفعيل الأربع زوايا للتكبير والتصغير بدل زاوية واحدة. تكبير المقابض المرئية بشكل بسيط.
- **اتصال الأونلاين:** رسائل مساعدة أوضح لخطأ 10061 ولما المستخدم يحاول يستخدم عنوان LAN بعيد عن البيت.
- **مزامنة الأونلاين v3:** أي تعديل في محتوى المشروع بيتبعت من خلال جهاز الهوست: رسم ومسح ودلو وإضافة أو حذف ليرات وفريمات وتعديل النص والتحويلات. إنشاء غرفة بيحوّل جهازك فعليًا لسيرفر TCP للغرفة، وبيظهر توضيح واضح في الواجهة.
- **إصلاح الدلو:** الخط اللي بيقسّم الدايرة بيمنع اللون من الانتقال للنصف التاني، مش بيتملى الشكل كله.
- **تحسين التايم لاين:** حذف أول فريم بيحافظ على الرسم اللي كان ظاهر في الفريم التاني، والفريمات اللي فيها نص أو Symbol بتبان مش فاضية.
- **حدود الأونلاين:** التعديلات المتزامنة على نفس العنصر ممكن تتعارض، والصور والصوت من ملفات خارجية مش بتتنقل، ولسه مطلوب تجربة فعلية بين جهازين Windows.
- أيقونات ملونة بتفاصيل أوضح لكل الأدوات الـ13 ومؤشرات الماوس، وأصوات UI جديدة مولّدة داخل البرنامج بدل الصفارات القديمة.
- تعبئة الدلو: طريقة إضافية بتحدد المساحات المقفولة من عدة خطوط مرسومة بالفرشة، وبترجع المنطقة لحدود فيكتور؛ مع استمرار التعبئة المباشرة للأشكال المغلقة. الحالات المعقدة لسه ليها حدود.
- أونلاين تجريبي على نفس الشبكة: الهوست بيشارك المشروع الحالي، وكود غرفة من 8 أرقام سهل النسخ، والانضمام بالكود محليًا أو IP، وقائمة بأسماء المشاركين ومستوياتهم اللي بيبلغوا بيها وألوانهم. ده مش دليل غرف عالمي، ولا تسجيل دخول Google معتمد؛ مزامنة الأدوات غير الرسم الجديد لسه مش كاملة.
- خانة المنظور: شبكة مربعات أو خطوط متساوية أو منظور بنقطة تلاشي، مع التحكم في المسافة واللون والشفافية والسمك ومكان نقطة التلاشي. الخطوط مساعدة ومش بتدخل في التصدير.
- التايم لاين يقدر يتمدد ويتحفظ ارتفاعه، وقسم الخصائص والليرز في اليمين بينهم فاصل قابل للسحب، وقسم الألوان والتماثل قابل للفتح والقفل.
- رجوع النسخ الاحتياطية بقى يعرض النسخة السابقة منفصلة عن عشر نسخ أقدم منها، من غير تكرار.
- **النسخة الاحتياطية:** خانة جديدة في ترس الإعدادات، تقدر ترجع منها لآخر إصدار أقدم أو تختار من 10 إصدارات قديمة متاحة بتاريخ نشر كل واحد. البرنامج بيتأكد من سلامة ملف EXE ببصمة SHA-256 وبيطلب تأكيد، ومش بيقفل لو في شغل لسه متحفظش.
- **لوحة الليرز:** مساحة أوسع قابلة للتمدد شبه برامج الرسم الاحترافية، ومعاينة فعلية صغيرة من خطوط كل لير، وبحث بالاسم، ومعلومات عن عدد الفريمات والخطوط، وخيارات للكليك يمين، بالإضافة لأزرار القفل والإظهار والنسخ والترتيب والشفافية. الكود مكتوب لـStick من غير نقل كود third-party drawing software.
- **أونلاين تجريبي:** اختيار جديد في شاشة البداية للرسم أو الأنيميشن سوا، مع غرف محلية وكلمة سر اختيارية وحد أقصى للأعضاء، وإظهار الغرف على نفس الشبكة أو الدخول بعنوان IP. مزامنة الخطوط الجديدة متاحة؛ التايم لاين والليرز مش متزامنين بالكامل، ومفيش دليل عالمي بالغرف أو تشفير كامل لسه.
- نظام الفنان (Art System) الجديد: شكل أنمي مع اسم تقدر تعدّله، وصورة شخصية من جهازك، وشريط المستوى والرتبة. تفاصيل احتساب التقدم مش ظاهرة.
- تخصيص مساحة الرسم، زووم ناحية الماوس، سهم تدوير حر للتحديد، تحريك التحديد الحر، وقائمة كليك يمين للأدوات، وست أشكال لرأس الفرشة.
- لما المستوى يعلى هتظهر نافذة حماسية في نص الشاشة 10 ثواني، وفيها رسالة من 1000 جملة مختلفة بالمصري أو الإنجليزي، ومعها صوت من 100 تنويعة مختلفة وخفيفة.
- ترس الإعدادات فيه تخصيص اختصارات الأدوات، وتغيير ألوان الواجهة والتايم لاين، وزر يرجّع الألوان الأصلية، وتحكم في أصوات البرنامج.
- قسم اليمين اتقسم لخصائص الأدوات والليرز كل واحد بسكرول مستقل. كليك يمين على اللير لتغيير اسمه أو نسخه أو لصقه أو دبلكيت أو حذفه.
- شكل التايم لاين اتحسّن، وتقدر تمدّه بسحب الحافة، وترس التايم لاين بيفتح إعدادات الأونين سكين من حيث العدد واللون والشفافية لقدام وورا.
- القلب الأحمر على الشمال بيفتح روابط يوتيوب وإنستجرام، والقلب الدهبي اللي في نص الشريط العلوي بيفتح دعم إنستا باي الاختياري.
- التحديد الحر بقى فيه مقبض لتكبير وتصغير الرسم، ونقاط الفيكتور تقدر تحركها، ومؤشر كل أداة بقى ليه شكله وصوته.
- الدلو بقى بيملأ كمان الرسومات الحرة المغلقة كويس، والتلوين بيظهر في الفيديو والصور المصدّرة.
- لما تدوس كليك يمين على فريم في التايم لاين، هتلاقي نسخ ولصق ومسح وحذف وكي فريم وكي فريم فاضي، مع حذف الزمن بشكل منظم من كل الليرز المفتوحة.
- الـ Onion Skin بقى فيه عدد فريمات وشفافية منفصلين للقديم والجديد، وكمان تقدر تحدد بداية ونهاية الـ Loop وتكبر التايم لاين.
- الليرز بقى فيها حذف بتأكيد، ونسخ وتحريك لفوق وتحت، وإظهار وقفل، وتحكم في الشفافية من صفر لـ 100%.
- أداة الأشكال بقى ليها حجم وشفافية ولون تعبئة اختياري. وحجم مؤشر الفرشاة والممحاة بقى بيتغير مع حجمهم، والخصائص بتتحرك لوحدها بالسكرول.
- البوكت بقى بيلوّن الأشكال الفيكتور المقفولة، والتحديد بقى مستطيل أو لاسو، والنص تقدر تحركه بأداة التحديد.
- أي نسخة جديدة بنضيفها لـ main وبتنجح في اختبارات وبناء Windows هتنزل تلقائيًا على GitHub Releases، عشان كل الناس تقدر تجربها.
- زرار «تحديث البرنامج» بقى متاح من الشاشة الرئيسية وكمان جوّه وضع الرسم والأنيميشن. ورقم الإصدار الحقيقي بقى متسجل جوّه Stick.exe نفسه.
- ضفنا رابط واضح للإبلاغ عن المشاكل واقتراح المميزات، ومعاه نماذج بسيطة عربي وإنجليزي.
- نافذة «إيه الجديد» بقت بتاخد التفاصيل من ملف الإصدار نفسه، وبتعرضها بالمصري أو إنجليزي أمريكي بسيط.
- تنزيل التحديث بقى على أجزاء صغيرة عشان يفضل خفيف على الأجهزة الضعيفة.
- طورنا وضع الرسم: ضغط قلم G-Pen، وشفافية الطبقات في التصدير، وأسماء الأدوات، وعرض الفريمات المجاورة في الأنيميشن.
- الرسم اللي خارج حدود الشاشة مش هيستهلك نفس المجهود في المعاينة.
- تقدر تدوس على «تحديثات» من شريط البرنامج. لو مفيش حاجة جديدة، Stick هيقولك إنك على آخر إصدار.
- Stick بيتأكد من سلامة ملف التحديث ببصمة SHA-256 قبل التثبيت.
- بعد ما التحديث يجهز، البرنامج بيعيد التشغيل، وبيوريك «إيه الجديد» أول مرة تفتح الإصدار الجديد.

### المشاكل اللي اتصلحت
- أداة نقاط الفيكتور بقت تسمح بتحديد وتحريك النقط.
- التحديد والتحويل بقى له مقبض مرئي لتغيير حجم الرسم والنص المحدد.
- الليرز مبقتش تتحرك مع سكرول قائمة الخصائص.
- البوكت كان بيتجاهل المساحات اللي مرسومة بالفرشة ومقفولة، وبعض الألوان كانت بتختفي من التصدير.
- احتساب نقاط XP بقى بيحفظ إنجاز كل فريم ولير وما بيكررش المكافأة بمجرد تكرار نفس الخط.
- صلحنا مشكلة ترتيب كلام رسالة «احفظ شغلك الأول» اللي كانت بتظهر قبل التحديث.
- صلحنا البوكت اللي كان بيلوّن خط قريب بدل المنطقة المقفولة اللي اخترتها.
- ضفنا حذف اللير وخلّينا تغييرات الشفافية والإظهار تتحفظ مع المشروع.
- شيلنا زر دمج قديم كان ممكن يضيّع محتوى الكي فريمات في ليرز تانية.
- صلحنا مشكلة إن النسخ الجديدة كانت بتتعمل على GitHub بس مبتتنشرش للمستخدمين كتحديثات.
- صلحنا ظهور رقم إصدار غلط لما ملف version.txt يكون قديم أو مش موجود.
- صلحنا مقارنة الإصدارات عشان نفس التحديث ميتعرضش كل شوية.
- صلحنا تعليق البحث عن تحديثات لما الاتصال بـGitHub يفشل.
- ضفنا تنبيه يحمي مشروعك لو عندك شغل لسه متحفظش.

> **For testers:** Every successful main build is published automatically. Please report issues using GitHub Issues. Back up important projects when testing new builds.

## Known limits / حدود النسخة التجريبية

- Bucket fill currently supports **closed vector shapes**. A genuine raster flood-fill brush-painted region is still being designed: https://github.com/KaniEGY/Stick/issues/15
- external drawing programs brush asset importing isn't shipping in this update; source formats/licenses and engine work are tracked here: https://github.com/KaniEGY/Stick/issues/14
- This public build is for testing. Save backups of important projects and report problems via GitHub Issues.

- **مهم:** البوكت دلوقتي بيملأ الأشكال الفيكتور المقفولة بس. تلوين أي مساحة مرسومة بالفرشة محتاج محرك Raster مستقل.
- فرش third-party drawing software وexternal animation software الأصلية لسه ما اتضافتش. هنستورد اللي ترخيصه يسمح بعد ما نجهز محرك فرش حقيقي ومتوافق.

### Technical limitations / حدود مهمة
- Bucket fill supports **closed drawn contours**, not full multi-layer pixel flood fill with tolerance; that is planned in Issue #15.
- The 100 level-up sounds are **lightweight synthesized patterns**, not bundled third-party sound files.
- The 1,000 level-up lines are 40 independently authored openings combined with 25 endings in each language.
- This release does not copy GPL code or artwork from third-party drawing software or external animation software; full brush-engine integration remains Issue #14.
- رتب وترقيات Hunter دي للمتعة والتشجيع بس؛ مالهاش تأثير على ملفات الرسم أو جودة التصدير.
