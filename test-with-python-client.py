#!/usr/bin/env python3
"""
Simple iSCSI client to test status codes with external tool.
Tests against the Rust iSCSI target implementation.
"""

import socket
import struct
import sys

def create_login_pdu(isid, tsih, cid, cmd_sn, exp_stat_sn, params):
    """Create an iSCSI Login Request PDU"""
    # BHS (Basic Header Segment) - 48 bytes
    opcode = 0x03  # LOGIN_REQUEST
    flags = 0x83  # Immediate + Transit + CSG=0 + NSG=1

    # Pad parameters to 4-byte boundary
    param_bytes = params.encode('utf-8')
    while len(param_bytes) % 4 != 0:
        param_bytes += b'\0'

    data_len = len(param_bytes)

    # Build BHS
    bhs = bytearray(48)
    bhs[0] = opcode | 0x40  # Immediate bit
    bhs[1] = flags
    # bytes 2-3: reserved
    bhs[4] = 0  # Total AHS length
    bhs[5] = (data_len >> 16) & 0xFF  # DataSegmentLength[0]
    bhs[6] = (data_len >> 8) & 0xFF   # DataSegmentLength[1]
    bhs[7] = data_len & 0xFF          # DataSegmentLength[2]

    # LUN (contains ISID + TSIH)
    bhs[8:14] = isid
    struct.pack_into('>H', bhs, 14, tsih)

    # ITT (Initiator Task Tag)
    struct.pack_into('>I', bhs, 16, cmd_sn)

    # Opcode-specific fields
    struct.pack_into('>H', bhs, 20, cid)  # CID
    struct.pack_into('>I', bhs, 24, cmd_sn)  # CmdSN
    struct.pack_into('>I', bhs, 28, exp_stat_sn)  # ExpStatSN

    return bytes(bhs) + param_bytes

def parse_login_response(data):
    """Parse iSCSI Login Response PDU"""
    if len(data) < 48:
        return None

    opcode = data[0] & 0x3F
    if opcode != 0x23:  # LOGIN_RESPONSE
        return None

    # Status is at bytes 36-37
    status_class = data[36]
    status_detail = data[37]

    return {
        'opcode': opcode,
        'status_class': status_class,
        'status_detail': status_detail,
        'status_code': (status_class << 8) | status_detail
    }

def test_target_not_found(host, port):
    """Test TARGET_NOT_FOUND (0x0203)"""
    print("Test 1: TARGET_NOT_FOUND (0x0203)...")

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5.0)

    try:
        sock.connect((host, port))

        # Send login with wrong target name
        params = "InitiatorName=iqn.2025-12.test:python-client\0"
        params += "TargetName=iqn.wrong.target.name\0"
        params += "AuthMethod=None\0"

        isid = b'\x01\x02\x03\x04\x05\x06'
        pdu = create_login_pdu(isid, 0, 0, 0, 0, params)

        sock.sendall(pdu)

        # Receive response
        response = sock.recv(48)
        result = parse_login_response(response)

        if result and result['status_code'] == 0x0203:
            print(f"  ✓ PASS: Received TARGET_NOT_FOUND (0x{result['status_code']:04x})")
            return True
        else:
            print(f"  ✗ FAIL: Expected 0x0203, got 0x{result['status_code']:04x if result else 0:04x}")
            return False
    except Exception as e:
        print(f"  ✗ FAIL: {e}")
        return False
    finally:
        sock.close()

def test_missing_parameter(host, port):
    """Test MISSING_PARAMETER (0x0207)"""
    print("Test 2: MISSING_PARAMETER (0x0207)...")

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5.0)

    try:
        sock.connect((host, port))

        # Send login WITHOUT InitiatorName (missing required parameter)
        params = "TargetName=iqn.2025-12.local:storage.memory-disk\0"
        params += "AuthMethod=None\0"

        isid = b'\x01\x02\x03\x04\x05\x07'
        pdu = create_login_pdu(isid, 0, 0, 0, 0, params)

        sock.sendall(pdu)

        # Receive response
        response = sock.recv(48)
        result = parse_login_response(response)

        if result and result['status_code'] == 0x0207:
            print(f"  ✓ PASS: Received MISSING_PARAMETER (0x{result['status_code']:04x})")
            return True
        else:
            print(f"  ✗ FAIL: Expected 0x0207, got 0x{result['status_code']:04x if result else 0:04x}")
            return False
    except Exception as e:
        print(f"  ✗ FAIL: {e}")
        return False
    finally:
        sock.close()

def test_successful_login(host, port):
    """Test SUCCESS (0x0000)"""
    print("Test 3: SUCCESS (0x0000)...")

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5.0)

    try:
        sock.connect((host, port))

        # Send valid login
        params = "InitiatorName=iqn.2025-12.test:python-client\0"
        params += "TargetName=iqn.2025-12.local:storage.memory-disk\0"
        params += "AuthMethod=None\0"

        isid = b'\x01\x02\x03\x04\x05\x08'
        pdu = create_login_pdu(isid, 0, 0, 0, 0, params)

        sock.sendall(pdu)

        # Receive response
        response = sock.recv(1024)  # May have parameters
        result = parse_login_response(response)

        if result and result['status_code'] == 0x0000:
            print(f"  ✓ PASS: Received SUCCESS (0x{result['status_code']:04x})")
            return True
        else:
            code = result['status_code'] if result else 0
            print(f"  ✗ FAIL: Expected 0x0000, got 0x{code:04x}")
            return False
    except Exception as e:
        print(f"  ✗ FAIL: {e}")
        return False
    finally:
        sock.close()

def main():
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <host> <port>")
        print(f"Example: {sys.argv[0]} 127.0.0.1 13260")
        sys.exit(1)

    host = sys.argv[1]
    port = int(sys.argv[2])

    print("=" * 50)
    print("iSCSI Python Client Test Suite")
    print("=" * 50)
    print(f"Testing target at {host}:{port}")
    print()

    results = []
    results.append(test_successful_login(host, port))
    results.append(test_target_not_found(host, port))
    results.append(test_missing_parameter(host, port))

    print()
    print("=" * 50)
    print(f"Results: {sum(results)}/{len(results)} tests passed")
    print("=" * 50)

    sys.exit(0 if all(results) else 1)

if __name__ == '__main__':
    main()
