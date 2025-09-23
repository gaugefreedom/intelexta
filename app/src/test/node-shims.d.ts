declare module "node:assert" {
  export function strictEqual<T>(actual: T, expected: T, message?: string | Error): void;
  export function deepStrictEqual<T>(actual: T, expected: T, message?: string | Error): void;
  export function ok(value: unknown, message?: string | Error): void;

  interface AssertModule {
    strictEqual: typeof strictEqual;
    deepStrictEqual: typeof deepStrictEqual;
    ok: typeof ok;
  }

  const assert: AssertModule;
  export default assert;
}

declare module "node:test" {
  interface TestOptions {
    signal?: AbortSignal;
    timeout?: number;
  }

  type TestFn = (t?: unknown) => void | Promise<void>;

  function test(fn: TestFn): Promise<void>;
  function test(name: string, fn: TestFn, options?: TestOptions): Promise<void>;
  function test(name: string, options: TestOptions, fn: TestFn): Promise<void>;

  export { test };
  export default test;
}
