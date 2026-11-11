// Pure SHA-256 hex digest helper with no business meaning (core/dry.md placement: shared/). Plan: ~/.claude/plans/hey-claude-we-have-gentle-ripple.md (copied from dexfs).
// Hashing is deterministic, so it is allowed in pure layers.
import { createHash } from 'node:crypto';

/**
 * Hashes UTF-8 text with SHA-256.
 * @param text - Input text, encoded as UTF-8.
 * @returns The 64-character lowercase hex digest.
 */
export const sha256Hex = (text: string): string =>
  createHash('sha256').update(text, 'utf8').digest('hex');
