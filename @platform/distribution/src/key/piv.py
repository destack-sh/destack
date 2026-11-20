import base64
import json
import sys
from datetime import datetime, timedelta, timezone

from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import padding
from ykman.device import list_all_devices
from ykman.piv import generate_self_signed_certificate, pivman_set_mgm_key
from yubikit.core.smartcard import ApduError, SmartCardConnection, SW
from yubikit.piv import (
    DEFAULT_MANAGEMENT_KEY, KEY_TYPE, MANAGEMENT_KEY_TYPE, PIN_POLICY,
    PivSession, SLOT, TOUCH_POLICY,
)


def slot_metadata(session):
    """Read the signature slot without treating transport failures as an empty slot."""
    try:
        return session.get_slot_metadata(SLOT.SIGNATURE)
    except ApduError as error:
        if error.sw != SW.REFERENCE_DATA_NOT_FOUND:
            raise

        return None


def pem(value):
    """Encode a public certificate or key."""
    if hasattr(value, "public_key"):
        return value.public_bytes(serialization.Encoding.PEM).decode()

    return value.public_bytes(
        serialization.Encoding.PEM, serialization.PublicFormat.SubjectPublicKeyInfo
    ).decode()


def execute(request):
    """Apply one PIV operation with secrets supplied only through the input pipe."""
    devices = [
        device for device, info in list_all_devices([SmartCardConnection])
        if str(info.serial) == request["serial"]
    ]
    if len(devices) != 1:
        raise ValueError("expected exactly one matching YubiKey")

    # connect only to the explicitly selected hardware serial
    with devices[0].open_connection(SmartCardConnection) as connection:
        session = PivSession(connection)
        slot = slot_metadata(session)
        if request["command"] == "inspect":
            return {
                "empty": slot is None,
                "defaultPin": session.get_pin_metadata().default_value,
                "defaultPuk": session.get_puk_metadata().default_value,
                "defaultManagement": session.get_management_key_metadata().default_value,
            }

        # sign standard RSA-PSS through the vendor library
        credential = request["credential"]
        if request["command"] == "sign":
            session.verify_pin(credential["pin"])
            signature = session.sign(
                SLOT.SIGNATURE, KEY_TYPE.RSA2048, base64.b64decode(request["message"]),
                hashes.SHA256(), padding.PSS(mgf=padding.MGF1(hashes.SHA256()), salt_length=32),
            )

            return {"signature": signature.hex()}

        if request["command"] != "enroll":
            raise ValueError("unknown PIV operation")

        # resume only known credentials, using metadata rather than guessing PINs
        if session.get_pin_metadata().default_value:
            session.change_pin("123456", credential["pin"])
        session.verify_pin(credential["pin"])
        if session.get_puk_metadata().default_value:
            session.change_puk("12345678", credential["puk"])

        # keep a recoverable management key in Bitwarden and protected on the device
        management = bytes.fromhex(credential["management"])
        current = session.get_management_key_metadata()
        session.authenticate(DEFAULT_MANAGEMENT_KEY if current.default_value else management)
        if current.default_value:
            pivman_set_mgm_key(
                session, management, MANAGEMENT_KEY_TYPE.AES256,
                touch=True, store_on_device=True,
            )

        # never replace an existing private key during an interrupted enrollment
        if slot is None:
            session.generate_key(
                SLOT.SIGNATURE, KEY_TYPE.RSA2048,
                PIN_POLICY.ALWAYS, TOUCH_POLICY.ALWAYS,
            )
        slot = session.get_slot_metadata(SLOT.SIGNATURE)
        if (
            slot.key_type != KEY_TYPE.RSA2048 or not slot.generated
            or slot.pin_policy != PIN_POLICY.ALWAYS
            or slot.touch_policy != TOUCH_POLICY.ALWAYS
        ):
            raise ValueError("unexpected signature key policy")

        # create the discovery certificate and retain public attestation records
        session.verify_pin(credential["pin"])
        now = datetime.now(timezone.utc)
        certificate = generate_self_signed_certificate(
            session, SLOT.SIGNATURE, slot.public_key,
            "CN=Destack root " + request["serial"], now, now + timedelta(days=3650),
        )
        session.put_certificate(SLOT.SIGNATURE, certificate)

        return {
            "public": pem(slot.public_key),
            "certificate": pem(certificate),
            "attestation": pem(session.attest_key(SLOT.SIGNATURE)),
            "issuer": pem(session.get_certificate(SLOT.ATTESTATION)),
        }


if __name__ == "__main__":
    try:
        print(json.dumps(execute(json.load(sys.stdin))))
    except Exception as error:
        # never include exception values that might contain credential input
        print("PIV operation failed: " + type(error).__name__, file=sys.stderr)
        sys.exit(1)
