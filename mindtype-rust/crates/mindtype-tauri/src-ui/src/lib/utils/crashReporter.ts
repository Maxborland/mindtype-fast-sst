/**
 * Crash reporter utilities for the frontend
 */

import { invoke } from '@tauri-apps/api/core';

export interface CrashReport {
  error_type: string;
  error_message: string;
  stack_trace?: string;
  user_description?: string;
}

/**
 * Submit a crash report to the backend
 */
export async function submitCrashReport(report: CrashReport): Promise<void> {
  try {
    await invoke('submit_crash_report', { report });
  } catch (e) {
    console.error('Failed to submit crash report:', e);
  }
}

/**
 * Get the path to the crash reports directory
 */
export async function getCrashReportsDir(): Promise<string> {
  return await invoke('get_crash_reports_dir');
}

/**
 * Capture and report an error
 */
export function captureError(error: Error, context?: string): void {
  const report: CrashReport = {
    error_type: error.name || 'Error',
    error_message: error.message,
    stack_trace: error.stack,
    user_description: context,
  };

  // Submit asynchronously, don't wait
  submitCrashReport(report).catch(() => {});
}

/**
 * Set up global error handlers for uncaught errors
 */
export function setupGlobalErrorHandlers(): void {
  // Handle uncaught errors
  window.addEventListener('error', (event) => {
    const report: CrashReport = {
      error_type: 'UncaughtError',
      error_message: event.message,
      stack_trace: event.error?.stack,
      user_description: `At ${event.filename}:${event.lineno}:${event.colno}`,
    };

    submitCrashReport(report).catch(() => {});
  });

  // Handle unhandled promise rejections
  window.addEventListener('unhandledrejection', (event) => {
    const error = event.reason;
    const report: CrashReport = {
      error_type: 'UnhandledPromiseRejection',
      error_message: error?.message || String(error),
      stack_trace: error?.stack,
    };

    submitCrashReport(report).catch(() => {});
  });
}
