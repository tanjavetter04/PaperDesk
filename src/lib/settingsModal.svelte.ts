export const settingsModal = $state({ open: false });

export function openSettingsModal(): void {
  settingsModal.open = true;
}

export function closeSettingsModal(): void {
  settingsModal.open = false;
}
