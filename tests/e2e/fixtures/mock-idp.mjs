#!/usr/bin/env node
// Local-only PR-12 identity/storage fixture. It mirrors the production HTTP
// contracts (ES256 JWT verification, Auth admin provisioning, and private
// Storage APIs) without adding a production bypass.
import http from 'node:http';
import crypto from 'node:crypto';

const PORT = Number(process.env.MOCK_IDP_PORT ?? 9100);
const ISSUER = `http://127.0.0.1:${PORT}/auth/v1`;
const KID = 'e2e-local-es256';
const FIXTURE_PASSWORD = 'e2e-password';
const KNOWLEDGE_BUCKET = 'edutalent-knowledge-sources';

const { privateKey, publicKey } = crypto.generateKeyPairSync('ec', { namedCurve: 'P-256' });
const publicJwk = publicKey.export({ format: 'jwk' });

const USERS = new Map([
  ['e2e-admin@example.test', 'b0000000-0000-0000-0000-0000000000a0'],
  ['e2e-manager-a@example.test', 'b0000000-0000-0000-0000-0000000000a1'],
  ['e2e-manager-b@example.test', 'b0000000-0000-0000-0000-0000000000b1'],
  ['e2e-teacher-a@example.test', 'b0000000-0000-0000-0000-0000000000a2'],
  ['e2e-teacher-b@example.test', 'b0000000-0000-0000-0000-0000000000b2'],
  ['e2e-student-a@example.test', 'b0000000-0000-0000-0000-0000000000a3'],
  ['e2e-student-b@example.test', 'b0000000-0000-0000-0000-0000000000b3'],
  ['e2e-parent-a@example.test', 'b0000000-0000-0000-0000-0000000000a4'],
  ['e2e-parent-b@example.test', 'b0000000-0000-0000-0000-0000000000b4'],
  ['e2e-inactive@example.test', 'b0000000-0000-0000-0000-0000000000a9'],
]);
const PASSWORDS = new Map([...USERS.keys()].map((email) => [email, FIXTURE_PASSWORD]));
const BUCKETS = new Map();
const OBJECTS = new Map();
let storageMode = 'ready';

const encode = (value) => Buffer.from(JSON.stringify(value)).toString('base64url');
function issueTokens(email, userId) {
  const now = Math.floor(Date.now() / 1000);
  const header = encode({ alg: 'ES256', typ: 'JWT', kid: KID });
  const claims = encode({
    sub: userId,
    email,
    aud: 'authenticated',
    iss: ISSUER,
    iat: now,
    exp: now + 900,
    role: 'authenticated',
  });
  const input = `${header}.${claims}`;
  const signature = crypto.sign('sha256', Buffer.from(input), {
    key: privateKey,
    dsaEncoding: 'ieee-p1363',
  }).toString('base64url');
  return {
    access_token: `${input}.${signature}`,
    token_type: 'bearer',
    expires_in: 900,
    refresh_token: `e2e-refresh-${userId}`,
    user: { id: userId, email, aud: 'authenticated' },
  };
}

function parseBody(body) {
  try {
    return JSON.parse(body || '{}');
  } catch {
    return null;
  }
}

function json(res, status, payload) {
  res.statusCode = status;
  res.setHeader('content-type', 'application/json');
  res.end(JSON.stringify(payload));
}

function storageUnavailable(res) {
  if (storageMode !== 'ready') {
    json(res, 503, { error: 'storage_unavailable' });
    return true;
  }
  return false;
}

const server = http.createServer((req, res) => {
  const url = new URL(req.url, `http://127.0.0.1:${PORT}`);
  const chunks = [];
  req.on('data', (chunk) => { chunks.push(chunk); });
  req.on('end', () => {
    const bodyBuffer = Buffer.concat(chunks);
    const body = bodyBuffer.toString('utf8');

    if (url.pathname === '/auth/v1/token' && url.searchParams.get('grant_type') === 'password') {
      const payload = parseBody(body);
      if (!payload) {
        json(res, 400, { error: 'invalid_request' });
        return;
      }
      const userId = USERS.get(payload.email);
      if (userId && payload.password === PASSWORDS.get(payload.email)) {
        json(res, 200, issueTokens(payload.email, userId));
      } else {
        json(res, 400, { error: 'invalid_grant', error_description: 'Invalid login credentials' });
      }
      return;
    }

    if (url.pathname === '/auth/v1/token' && url.searchParams.get('grant_type') === 'refresh_token') {
      const payload = parseBody(body);
      const entry = payload && [...USERS.entries()].find(([, userId]) =>
        `e2e-refresh-${userId}` === payload.refresh_token,
      );
      if (entry) {
        json(res, 200, issueTokens(entry[0], entry[1]));
      } else {
        json(res, 400, { error: 'invalid_grant' });
      }
      return;
    }

    if (req.method === 'POST' && url.pathname === '/auth/v1/admin/users') {
      const payload = parseBody(body);
      const email = payload?.email?.trim()?.toLowerCase();
      const password = payload?.password;
      if (!email || typeof password !== 'string' || password.length === 0) {
        json(res, 400, { message: 'email and password are required' });
        return;
      }
      if (USERS.has(email)) {
        json(res, 422, { message: 'User already registered' });
        return;
      }

      const id = crypto.randomUUID();
      USERS.set(email, id);
      PASSWORDS.set(email, password);
      json(res, 200, {
        id,
        email,
        created_at: new Date().toISOString(),
        user_metadata: payload.user_metadata ?? {},
        app_metadata: payload.app_metadata ?? null,
      });
      return;
    }

    if (req.method === 'DELETE' && url.pathname.startsWith('/auth/v1/admin/users/')) {
      const userId = decodeURIComponent(url.pathname.slice('/auth/v1/admin/users/'.length));
      const entry = [...USERS.entries()].find(([, id]) => id === userId);
      if (!entry) {
        json(res, 404, { message: 'User not found' });
        return;
      }
      USERS.delete(entry[0]);
      PASSWORDS.delete(entry[0]);
      json(res, 200, { id: userId });
      return;
    }

    if (url.pathname === '/auth/v1/.well-known/jwks.json') {
      json(res, 200, {
        keys: [{
          kty: 'EC', crv: 'P-256', alg: 'ES256', use: 'sig', kid: KID,
          x: publicJwk.x, y: publicJwk.y,
        }],
      });
      return;
    }

    // Test-control endpoint is exposed only by this local fixture process. It
    // lets Playwright exercise truthful retryable-storage states without a
    // production-only flag or network fault injection in application code.
    if (url.pathname === '/__e2e/storage-mode') {
      if (req.method === 'POST') {
        const payload = parseBody(body);
        if (!payload || !['ready', 'unavailable'].includes(payload.mode)) {
          json(res, 400, { error: 'invalid_storage_mode' });
          return;
        }
        storageMode = payload.mode;
      }
      json(res, 200, { mode: storageMode });
      return;
    }

    if (req.method === 'GET' && url.pathname.startsWith('/storage/v1/bucket/')) {
      if (storageUnavailable(res)) return;
      const bucketId = decodeURIComponent(url.pathname.slice('/storage/v1/bucket/'.length));
      const bucket = BUCKETS.get(bucketId);
      if (!bucket) {
        json(res, 404, { error: 'not_found' });
        return;
      }
      json(res, 200, bucket);
      return;
    }

    if (req.method === 'POST' && url.pathname === '/storage/v1/bucket') {
      if (storageUnavailable(res)) return;
      const payload = parseBody(body);
      const bucketId = payload?.id;
      if (!bucketId || typeof bucketId !== 'string') {
        json(res, 400, { error: 'invalid_bucket' });
        return;
      }
      if (BUCKETS.has(bucketId)) {
        json(res, 409, { error: 'already_exists' });
        return;
      }
      const bucket = {
        id: bucketId,
        name: payload.name ?? bucketId,
        public: payload.public === true,
      };
      BUCKETS.set(bucketId, bucket);
      json(res, 200, bucket);
      return;
    }

    const objectPrefix = `/storage/v1/object/${KNOWLEDGE_BUCKET}/`;
    if (req.method === 'POST' && url.pathname.startsWith(objectPrefix)) {
      if (storageUnavailable(res)) return;
      if (!BUCKETS.has(KNOWLEDGE_BUCKET)) {
        json(res, 404, { error: 'bucket_not_found' });
        return;
      }
      const objectKey = decodeURIComponent(url.pathname.slice(objectPrefix.length));
      if (!objectKey) {
        json(res, 400, { error: 'invalid_object_key' });
        return;
      }
      const storageKey = `${KNOWLEDGE_BUCKET}/${objectKey}`;
      if (OBJECTS.has(storageKey) && req.headers['x-upsert'] !== 'true') {
        json(res, 409, { error: 'already_exists' });
        return;
      }
      OBJECTS.set(storageKey, Buffer.from(bodyBuffer));
      json(res, 200, { key: storageKey });
      return;
    }

    if (req.method === 'DELETE' && url.pathname === `/storage/v1/object/${KNOWLEDGE_BUCKET}`) {
      if (storageUnavailable(res)) return;
      const payload = parseBody(body);
      for (const prefix of payload?.prefixes ?? []) {
        OBJECTS.delete(`${KNOWLEDGE_BUCKET}/${prefix}`);
      }
      json(res, 200, { deleted: payload?.prefixes ?? [] });
      return;
    }

    json(res, 404, { error: 'not_found' });
  });
});

server.listen(PORT, '127.0.0.1', () => {
  console.log(`mock-idp listening on ${ISSUER}`);
});
