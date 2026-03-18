import json
import base64
import hashlib
from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.exceptions import InvalidSignature

# Token George sent (24h)
data_orig = {
    "schema": "chora.continuation.token.v1",
    "ledger_event_id": "763331c3-3b82-485d-b449-0f6f033a5203",
    "source_capsule_root": "ef7e9de0b541489e249ce4f7c6f49c078d5537be512592c52215e7441222037d",
    "resolution_event_id": "ems-resolve-763331c3-3b82-485d-b449-0f6f033a5203",
    "nonce": "nonce-24h-test-001",
    "iat": "2026-03-18T03:24:14.780283+00:00",
    "exp": "2026-03-19T03:24:14.780300+00:00",
    "issuer": "chora-gate-v0.3"
}

sig_hex = "f8853c9a14df9be9bfc603553ece9fe4bd379a6effeb5cdd07e6f6fabf6f5971299544aab99a681c33f146ad1e5b9c6dc6a7d263d1aadf2dddcf1510dd3fcb0d"
pk_hex = "e349f4640029c01b52745c6a41fe4b7a13b408eda008d38570c3baeb8c45a189"

sig = bytes.fromhex(sig_hex)
pk = ed25519.Ed25519PublicKey.from_public_bytes(bytes.fromhex(pk_hex))

def try_verify(label, bytes_data):
    try:
        pk.verify(sig, bytes_data)
        print(f"!!! SUCCESS !!! {label}")
        print(f"Bytes (hex): {bytes_data.hex()}")
        return True
    except InvalidSignature:
        return False

# Ordering: JCS (Alphabetical) vs Original
keys_jcs = sorted(data_orig.keys())
keys_orig = ["schema", "ledger_event_id", "source_capsule_root", "resolution_event_id", "nonce", "iat", "exp", "issuer"]

# Variation: Omitting resolution_event_id entirely
keys_omit = [k for k in keys_orig if k != "resolution_event_id"]
keys_jcs_omit = [k for k in keys_jcs if k != "resolution_event_id"]

timezones = ["+00:00", "Z", ""]

for keys in [keys_jcs, keys_orig, keys_omit, keys_jcs_omit]:
    key_label = "JCS" if keys == keys_jcs else "ORIG"
    if keys == keys_omit: key_label = "OMIT_ORIG"
    if keys == keys_jcs_omit: key_label = "OMIT_JCS"
    for tz_iat in timezones:
        for tz_exp in timezones:
            p = data_orig.copy()
            p["iat"] = "2026-03-18T03:24:14.780283" + tz_iat
            p["exp"] = "2026-03-19T03:24:14.780300" + tz_exp
            
            # Formatting: Tight vs Spaced
            formats = [
                ( (',', ':'), "Tight" ),
                ( (', ', ': '), "Spaces" )
            ]
            
            for (seps, fmt_label) in formats:
                # Construct JSON string with specific key order
                segments = []
                for k in keys:
                    v = p[k]
                    v_str = json.dumps(v) # Handled quotes/null correctly
                    segments.append(f"{json.dumps(k)}{seps[1]}{v_str}")
                
                json_str = "{" + seps[0].join(segments) + "}"
                raw_bytes = json_str.encode('utf-8')
                
                full_label = f"{key_label} | {fmt_label} | iat_tz={tz_iat} | exp_tz={tz_exp}"
                
                # Try raw
                if try_verify(f"{full_label} | RAW", raw_bytes): exit(0)
                
                # Try SHA256 of raw
                h = hashlib.sha256(raw_bytes).digest()
                if try_verify(f"{full_label} | SHA256", h): exit(0)
                
                # Try Base64 of raw
                b64 = base64.b64encode(raw_bytes)
                if try_verify(f"{full_label} | BASE64", b64): exit(0)

print("Brute force failed for all common permutations.")
