#!/usr/bin/env node
/**
 * Generate Ed25519 keypair for signing verifiable bundles
 */

import nacl from 'tweetnacl';
import util from 'tweetnacl-util';

const { encodeBase64 } = util;

const keypair = nacl.sign.keyPair();

console.log('\n╔════════════════════════════════════════════════════════════╗');
console.log('║  Ed25519 Keypair Generated                                 ║');
console.log('╚════════════════════════════════════════════════════════════╝\n');

console.log('Public Key (share this):');
console.log(encodeBase64(keypair.publicKey));
console.log('');

console.log('Secret Key (keep this private):');
console.log(encodeBase64(keypair.secretKey));
console.log('');

console.log('Add to your .env file:');
console.log(`ED25519_SECRET_KEY=${encodeBase64(keypair.secretKey)}`);
console.log('');
