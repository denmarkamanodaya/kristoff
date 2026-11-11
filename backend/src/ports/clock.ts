// Clock port: the only source of the current time for the core (core/architecture-hexagonal.md).

/** Reads the current time. */
export interface Clock {
  /** Epoch milliseconds now. */
  now(): number;
}
