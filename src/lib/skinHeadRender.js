// Renders an account's head as a small isometric-style PNG icon using the real skinview3d model instead of the
// old hand-rolled canvas/UV-cropping approach, which visibly dropped the head's "hat" overlay layer on some skins.

// Each icon renders once into an offscreen SkinViewer, snapshots to a PNG data URL, then disposes the viewer
// immediately; a full WebGL context per avatar risks hitting the browser's simultaneous-context limit. Cached by skin+size+angle.

// Dynamically imported so the always-mounted Topbar avatar doesn't drag skinview3d/three.js into the main
// bundle; costs a ~400KB chunk fetch on first avatar render, but as its own cacheable, non-blocking chunk.
let skinview3dPromise = null;
function loadSkinview3d() {
  if (!skinview3dPromise) skinview3dPromise = import('skinview3d');
  return skinview3dPromise;
}

// skinview3d's own model.js places the head's world center at y=12 (SkinObject is offset +8 within PlayerObject,
// plus the head box's own local center at y=4); the outer "hat" layer box is 9x9x9, the largest extent that needs to fit in frame.
const HEAD_CENTER_Y = 12;
const HEAD_SIZE = 9;
const FOV = 25;
// A shallow angle rather than a true 45°/35° isometric, enough to read as a 3D cube while keeping the face
// itself the dominant, clearly-recognizable thing in a tiny avatar.
const AZIMUTH = Math.PI / 8; // 22.5°
const ELEVATION = Math.PI / 14; // ~12.9°

const cache = new Map();

async function renderOnce(skinUrl, sizePx, frontFace) {
  const { SkinViewer } = await loadSkinview3d();
  const canvas = document.createElement('canvas');
  const viewer = new SkinViewer({
    canvas,
    width: sizePx,
    height: sizePx,
    fov: FOV,
    enableControls: false,
  });

  return viewer
    .loadSkin(skinUrl, { model: 'default' })
    .then(() => {
      const skin = viewer.playerObject.skin;
      skin.body.visible = false;
      skin.leftArm.visible = false;
      skin.rightArm.visible = false;
      skin.leftLeg.visible = false;
      skin.rightLeg.visible = false;
      viewer.playerObject.cape.visible = false;
      viewer.playerObject.elytra.visible = false;
      viewer.playerObject.ears.visible = false;

      const fovRad = (FOV * Math.PI) / 180;
      const margin = 1.5;
      const distance = (HEAD_SIZE * margin) / 2 / Math.tan(fovRad / 2);
      const azimuth = (frontFace === 'left' ? -1 : 1) * AZIMUTH;

      const x = distance * Math.cos(ELEVATION) * Math.sin(azimuth);
      const y = HEAD_CENTER_Y + distance * Math.sin(ELEVATION);
      const z = distance * Math.cos(ELEVATION) * Math.cos(azimuth);
      viewer.camera.position.set(x, y, z);
      viewer.camera.lookAt(0, HEAD_CENTER_Y, 0);
      viewer.controls.target.set(0, HEAD_CENTER_Y, 0);
      viewer.controls.update();

      viewer.render();
      const dataUrl = canvas.toDataURL('image/png');
      viewer.dispose();
      return dataUrl;
    })
    .catch((e) => {
      viewer.dispose();
      throw e;
    });
}

export function renderSkinHeadIcon(skinUrl, sizePx, frontFace = 'right') {
  const key = `${skinUrl}::${sizePx}::${frontFace}`;
  if (!cache.has(key)) {
    const promise = renderOnce(skinUrl, sizePx, frontFace);
    promise.catch(() => cache.delete(key)); // don't poison the cache with a failed attempt
    cache.set(key, promise);
  }
  return cache.get(key);
}
