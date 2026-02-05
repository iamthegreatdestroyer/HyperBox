/**
 * Mock for Tauri plugins
 * Provides mock implementations for Tauri plugin calls in tests
 */

import { vi } from "vitest";

// @tauri-apps/plugin-os
export const platform = vi.fn(async () => "windows");
export const arch = vi.fn(async () => "x86_64");
export const version = vi.fn(async () => "10.0.22621");
export const type = vi.fn(async () => "Windows_NT");
export const hostname = vi.fn(async () => "test-machine");
export const locale = vi.fn(async () => "en-US");

// @tauri-apps/plugin-shell
export const open = vi.fn(async () => {});
export const Command = vi.fn().mockImplementation(() => ({
  spawn: vi.fn(async () => ({
    write: vi.fn(),
    kill: vi.fn(),
  })),
  execute: vi.fn(async () => ({
    code: 0,
    stdout: "",
    stderr: "",
  })),
}));

// @tauri-apps/plugin-dialog
export const openDialog = vi.fn(async () => null);
export const saveDialog = vi.fn(async () => null);
export const messageDialog = vi.fn(async () => true);
export const askDialog = vi.fn(async () => true);
export const confirmDialog = vi.fn(async () => true);

// Alias exports for different import patterns
export const openPath = open;
export const save = saveDialog;
export const message = messageDialog;
export const ask = askDialog;
export const confirm = confirmDialog;

// @tauri-apps/plugin-fs
export const readTextFile = vi.fn(async () => "");
export const writeTextFile = vi.fn(async () => {});
export const readBinaryFile = vi.fn(async () => new Uint8Array());
export const writeBinaryFile = vi.fn(async () => {});
export const readDir = vi.fn(async () => []);
export const createDir = vi.fn(async () => {});
export const removeDir = vi.fn(async () => {});
export const removeFile = vi.fn(async () => {});
export const renameFile = vi.fn(async () => {});
export const copyFile = vi.fn(async () => {});
export const exists = vi.fn(async () => false);

// @tauri-apps/plugin-notification
export const isPermissionGranted = vi.fn(async () => true);
export const requestPermission = vi.fn(async () => "granted");
export const sendNotification = vi.fn(async () => {});

// @tauri-apps/plugin-process
export const exit = vi.fn(async () => {});
export const relaunch = vi.fn(async () => {});
