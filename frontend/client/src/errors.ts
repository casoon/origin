import type { ErrorContract, ErrorKind } from "./types";

/**
 * A failure that crossed the IPC boundary.
 *
 * Every rejected command produces one of these, so views can branch on `kind`
 * instead of matching on message strings.
 */
export class OriginError extends Error {
  readonly kind: ErrorKind;
  readonly retryable: boolean;
  readonly needsUserAction: boolean;
  readonly retryAfterSeconds: number | null;

  constructor(contract: ErrorContract) {
    super(contract.message);
    this.name = "OriginError";
    this.kind = contract.kind;
    this.retryable = contract.retryable;
    this.needsUserAction = contract.needs_user_action;
    this.retryAfterSeconds = contract.retry_after_seconds;
  }
}

function isErrorContract(value: unknown): value is ErrorContract {
  return (
    typeof value === "object" &&
    value !== null &&
    "kind" in value &&
    "message" in value &&
    typeof (value as ErrorContract).message === "string"
  );
}

/**
 * Normalise whatever a rejected invoke threw.
 *
 * Anything that is not a contract means a bug in the host layer — a command leaked a
 * raw error. It is reported as `internal` rather than shown to the user verbatim.
 */
export function toOriginError(thrown: unknown): OriginError {
  if (thrown instanceof OriginError) {
    return thrown;
  }

  if (isErrorContract(thrown)) {
    return new OriginError(thrown);
  }

  return new OriginError({
    kind: "internal",
    message: thrown instanceof Error ? thrown.message : String(thrown),
    retryable: false,
    needs_user_action: false,
    retry_after_seconds: null,
  });
}
