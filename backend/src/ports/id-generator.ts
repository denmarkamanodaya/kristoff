// IdGenerator port: ids and randomness come from outside the core (core/architecture-hexagonal.md).
// Contract: docs/contract.md (ids 16 lowercase hex; message ids ULID, 26 chars).

/** Makes new ids. */
export interface IdGenerator {
  /**
   * A new doc or chat id: 16 lowercase hex characters that sort by creation time (12 hex of epoch
   * milliseconds, then 4 hex), so sort keys built from it order items oldest to newest.
   */
  newId(): string;
  /** A new message id: a ULID (26 Crockford base32 characters) that sorts by time. */
  newUlid(): string;
}
