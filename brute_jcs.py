import json
import ed25519
import binascii

# George's 24-hour token sample
payload = {
    "schema": "chora.continuation.token.v1",
    "ledger_event_id": "763331c3-3b82-485d-b449-0f6f033a5203",
    "source_capsule_root": "ef7e9de0b541489e249ce4f7c6f49c078d5537be512592c52215e7441222037d",
    "resolution_event_id": "ems-resolve-763331c3-3b82-485d-b449-0f6f033a5203",
    "nonce": "nonce-24h-test-001",
    "iat": "2026-03-18T03:24:14.780283+00:00",
    "exp": "2026-03-19T03:24:14.780300+00:00",
    "issuer": "chora-gate-v0.3"
}

signature_hex = "f8853c9a14df9be9bfc603553ece9fe4bd379a6effeb5cdd07e6f6fabf6f5971299544aab99a681c33f146ad1e5b9c6dc6a7d263d1aadf2dddcf1510dd3fcb0d"
public_key_hex = "e349f4640029c01b52745c6a41fe4b7a13b408eda008d38570c3baeb8c45a189"

signature = binascii.unhexlify(signature_hex)
public_key = binascii.unhexlify(public_key_hex)
vk = ed25519.VerifyingKey(public_key)

def jcs_serialize(obj):
    # Sort keys and remove whitespace
    return json.dumps(obj, sort_keys=True, separators=(',', ':')).encode('utf-8')

# Permutations
timezones = ["+00:00", "Z", ""]
precisions = [0, 3, 6] # None, milliseconds, microseconds

for tz_iat in timezones:
    for tz_exp in timezones:
        # Try exact strings from sample
        p = payload.copy()
        p["iat"] = "2026-03-18T03:24:14.780283" + tz_iat
        p["exp"] = "2026-03-19T03:24:14.780300" + tz_exp
        
        jcs = jcs_serialize(p)
        try:
            vk.verify(signature, jcs)
            print(f"SUCCESS with TZ iat={tz_iat}, exp={tz_exp}: {jcs.decode()}")
            exit(0)
        except ed25519.BadSignatureError:
            pass
            
        # Try truncated exp (7803 instead of 780300)
        p["exp"] = "2026-03-19T03:24:14.7803" + tz_exp
        jcs = jcs_serialize(p)
        try:
            vk.verify(signature, jcs)
            print(f"SUCCESS with TZ iat={tz_iat}, truncated exp={p['exp']}: {jcs.decode()}")
            exit(0)
        except ed25519.BadSignatureError:
            pass

print("No match found in basic permutations.")
