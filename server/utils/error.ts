export class ExtendableError extends Error {
  constructor(message?: string) {
    super(message);
    Object.setPrototypeOf(this, new.target.prototype);
    this.name = new.target.name;
  }
}

export class SafeUnreachableError extends ExtendableError {
  constructor(value: never) {
    super(`Unreachable: ${JSON.stringify(value)}`);
  }
}
