// apps/web-verifier/src/wasm/wasm-module.d.ts
declare module '/pkg/*.js' {
  const mod: any;
  export default mod;
  export function init_verifier(): void;
  export function verify_car_bytes(bytes: Uint8Array): Promise<any>;
  export function verify_car_json(json: string): Promise<any>;
}
