export class NvgApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly details: Record<string, unknown>;

  constructor(status: number, code: string, message: string, details: Record<string, unknown> = {}) {
    super(message);
    this.name = "NvgApiError";
    this.status = status;
    this.code = code;
    this.details = details;
  }
}

export class NvgNetworkError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "NvgNetworkError";
  }
}
