// Centralized icon set for context-menu items so the same action always gets the same glyph everywhere.
// Raw path/shape markup only, never user input, rendered via v-html in ContextMenu.vue.
export const MENU_ICONS = {
  play: '<path d="M7 5l12 7-12 7z" fill="currentColor" stroke="none"/>',
  stop: '<rect x="6" y="6" width="12" height="12" rx="1.5" fill="currentColor" stroke="none"/>',
  folder: '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z"/>',
  settings:
    '<circle cx="12" cy="12" r="3.2"/><path d="M19.4 13.5a7.6 7.6 0 0 0 0-3l1.9-1.5-2-3.4-2.3.6a7.6 7.6 0 0 0-2.6-1.5L14 2.5h-4l-.4 2.2a7.6 7.6 0 0 0-2.6 1.5l-2.3-.6-2 3.4L4.6 10.5a7.6 7.6 0 0 0 0 3l-1.9 1.5 2 3.4 2.3-.6a7.6 7.6 0 0 0 2.6 1.5l.4 2.2h4l.4-2.2a7.6 7.6 0 0 0 2.6-1.5l2.3.6 2-3.4z"/>',
  trash: '<path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2m3 0-1 14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2L4 6h16Z"/>',
  upload: '<path d="M12 3v12m0-12 4 4m-4-4-4 4M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2"/>',
  rotate: '<path d="M3 12a9 9 0 1 0 3-6.7M3 4v5h5"/>',
  history: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 3"/>',
  copy: '<rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>',
  power: '<path d="M12 2v10"/><path d="M18.4 6.6a9 9 0 1 1-12.77.04"/>',
  eye: '<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/>',
  edit: '<path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4Z"/>',
};
