import { invoke } from '@tauri-apps/api/core';

export const getDefaultSkin = () => invoke('get_default_skin');

export const getSkinProfile = () => invoke('get_skin_profile');

export const uploadSkin = (variant, bytes) => invoke('upload_skin', { variant, bytes });

export const resetSkin = () => invoke('reset_skin');

export const equipCape = (capeId) => invoke('equip_cape', { capeId });

export const unequipCape = () => invoke('unequip_cape');

export const setSkinVariant = (variant) => invoke('set_skin_variant', { variant });
