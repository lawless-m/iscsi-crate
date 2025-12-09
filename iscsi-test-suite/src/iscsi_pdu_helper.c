#include "iscsi_pdu_helper.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <netdb.h>

/* iSCSI PDU Opcodes */
#define ISCSI_OPCODE_LOGIN_REQUEST 0x03
#define ISCSI_OPCODE_LOGIN_RESPONSE 0x23

/* Login flags */
#define ISCSI_LOGIN_FLAG_TRANSIT 0x80
#define ISCSI_LOGIN_FLAG_CONTINUE 0x40
#define ISCSI_LOGIN_FLAG_CSG_MASK 0x0C
#define ISCSI_LOGIN_FLAG_NSG_MASK 0x03

/* CSG/NSG values: 0=SecurityNegotiation, 1=LoginOperationalNegotiation, 2=FullFeaturePhase */
#define ISCSI_NSG_SECURITY 0
#define ISCSI_NSG_OPERATIONAL 1
#define ISCSI_NSG_FFP 2

/* Helper to encode 24-bit big-endian integer */
static void encode_24bit(uint8_t *buf, uint32_t value) {
    buf[0] = (value >> 16) & 0xFF;
    buf[1] = (value >> 8) & 0xFF;
    buf[2] = value & 0xFF;
}

/* Helper to encode 32-bit big-endian integer */
static void encode_32bit(uint8_t *buf, uint32_t value) {
    buf[0] = (value >> 24) & 0xFF;
    buf[1] = (value >> 16) & 0xFF;
    buf[2] = (value >> 8) & 0xFF;
    buf[3] = value & 0xFF;
}

/* Helper to encode 64-bit big-endian integer (not currently used) */
__attribute__((unused))
static void encode_64bit(uint8_t *buf, uint64_t value) {
    buf[0] = (value >> 56) & 0xFF;
    buf[1] = (value >> 48) & 0xFF;
    buf[2] = (value >> 40) & 0xFF;
    buf[3] = (value >> 32) & 0xFF;
    buf[4] = (value >> 24) & 0xFF;
    buf[5] = (value >> 16) & 0xFF;
    buf[6] = (value >> 8) & 0xFF;
    buf[7] = value & 0xFF;
}

/**
 * Build a key-value data segment for login negotiation
 * Format: "Key=Value\0" padded to 4-byte boundary
 */
static int build_kv_segment(uint8_t *buf, size_t max_size,
                            const iscsi_kv_pair_t *pairs, int num_pairs) {
    size_t offset = 0;

    for (int i = 0; i < num_pairs; i++) {
        size_t pair_len = strlen(pairs[i].key) + 1 + strlen(pairs[i].value) + 1;
        if (offset + pair_len > max_size) {
            return -1;
        }

        /* Write "Key=Value\0" */
        strcpy((char *)buf + offset, pairs[i].key);
        offset += strlen(pairs[i].key);
        buf[offset++] = '=';
        strcpy((char *)buf + offset, pairs[i].value);
        offset += strlen(pairs[i].value);
        buf[offset++] = '\0';
    }

    /* Pad to 4-byte boundary */
    size_t padded_size = (offset + 3) & ~3;
    memset(buf + offset, 0, padded_size - offset);

    return padded_size;
}

/**
 * Build Login Request PDU with invalid MaxRecvDataSegmentLength (value=0, which is invalid)
 */
uint8_t* build_login_pdu_invalid_maxrecvdatasize(size_t *pdu_size) {
    uint8_t *pdu;
    iscsi_bhs_t *bhs;
    iscsi_kv_pair_t pairs[5];
    int num_pairs = 0;
    uint8_t *data_segment;
    int data_size;
    size_t total_size;

    /* Build key-value pairs with INVALID MaxRecvDataSegmentLength=0 */
    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "InitiatorName");
    strcpy(pairs[num_pairs].value, "iqn.2024-12.com.test:initiator");
    num_pairs++;

    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "TargetName");
    strcpy(pairs[num_pairs].value, "iqn.2024-12.com.test:target");
    num_pairs++;

    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "SessionType");
    strcpy(pairs[num_pairs].value, "Normal");
    num_pairs++;

    /* INVALID: MaxRecvDataSegmentLength=0 (RFC 3720 requires > 512) */
    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "MaxRecvDataSegmentLength");
    strcpy(pairs[num_pairs].value, "0");
    num_pairs++;

    /* Allocate space for BHS (48) + data segment */
    data_segment = (uint8_t *)malloc(1024);
    if (!data_segment) {
        return NULL;
    }

    data_size = build_kv_segment(data_segment, 1024, pairs, num_pairs);
    if (data_size < 0) {
        free(data_segment);
        return NULL;
    }

    total_size = 48 + data_size;
    pdu = (uint8_t *)malloc(total_size);
    if (!pdu) {
        free(data_segment);
        return NULL;
    }

    /* Build BHS */
    memset(pdu, 0, 48);
    bhs = (iscsi_bhs_t *)pdu;

    bhs->opcode = ISCSI_OPCODE_LOGIN_REQUEST;
    bhs->flags = ISCSI_LOGIN_FLAG_TRANSIT | ISCSI_NSG_OPERATIONAL;
    bhs->version_max = 0x00;
    bhs->version_active = 0x00;
    encode_24bit(pdu + 1, data_size);  /* Length (24-bit at offset 1) */
    encode_32bit(pdu + 8, 0);           /* ISID - 6 bytes starting at offset 8 */
    encode_32bit(pdu + 16, 1);          /* Init Task Tag */
    encode_32bit(pdu + 24, 0);          /* CmdSN */
    encode_32bit(pdu + 28, 0);          /* ExpStatSN */

    /* Copy data segment */
    memcpy(pdu + 48, data_segment, data_size);
    free(data_segment);

    *pdu_size = total_size;
    return pdu;
}

/**
 * Build Login Request PDU with invalid MaxConnections (value=0, which is invalid)
 */
uint8_t* build_login_pdu_invalid_maxconnections(size_t *pdu_size) {
    uint8_t *pdu;
    iscsi_bhs_t *bhs;
    iscsi_kv_pair_t pairs[5];
    int num_pairs = 0;
    uint8_t *data_segment;
    int data_size;
    size_t total_size;

    /* Build key-value pairs with INVALID MaxConnections=0 */
    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "InitiatorName");
    strcpy(pairs[num_pairs].value, "iqn.2024-12.com.test:initiator");
    num_pairs++;

    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "TargetName");
    strcpy(pairs[num_pairs].value, "iqn.2024-12.com.test:target");
    num_pairs++;

    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "SessionType");
    strcpy(pairs[num_pairs].value, "Normal");
    num_pairs++;

    /* INVALID: MaxConnections=0 (RFC 3720 requires >= 1) */
    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "MaxConnections");
    strcpy(pairs[num_pairs].value, "0");
    num_pairs++;

    /* Allocate space for BHS (48) + data segment */
    data_segment = (uint8_t *)malloc(1024);
    if (!data_segment) {
        return NULL;
    }

    data_size = build_kv_segment(data_segment, 1024, pairs, num_pairs);
    if (data_size < 0) {
        free(data_segment);
        return NULL;
    }

    total_size = 48 + data_size;
    pdu = (uint8_t *)malloc(total_size);
    if (!pdu) {
        free(data_segment);
        return NULL;
    }

    /* Build BHS */
    memset(pdu, 0, 48);
    bhs = (iscsi_bhs_t *)pdu;

    bhs->opcode = ISCSI_OPCODE_LOGIN_REQUEST;
    bhs->flags = ISCSI_LOGIN_FLAG_TRANSIT | ISCSI_NSG_OPERATIONAL;
    bhs->version_max = 0x00;
    bhs->version_active = 0x00;
    encode_24bit(pdu + 1, data_size);
    encode_32bit(pdu + 16, 2);  /* Different Init Task Tag */
    encode_32bit(pdu + 24, 0);
    encode_32bit(pdu + 28, 0);

    /* Copy data segment */
    memcpy(pdu + 48, data_segment, data_size);
    free(data_segment);

    *pdu_size = total_size;
    return pdu;
}

/**
 * Build Login Request PDU with contradictory parameter combination
 */
uint8_t* build_login_pdu_invalid_param_combo(size_t *pdu_size) {
    uint8_t *pdu;
    iscsi_bhs_t *bhs;
    iscsi_kv_pair_t pairs[6];
    int num_pairs = 0;
    uint8_t *data_segment;
    int data_size;
    size_t total_size;

    /* Build key-value pairs with contradictory settings */
    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "InitiatorName");
    strcpy(pairs[num_pairs].value, "iqn.2024-12.com.test:initiator");
    num_pairs++;

    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "TargetName");
    strcpy(pairs[num_pairs].value, "iqn.2024-12.com.test:target");
    num_pairs++;

    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "SessionType");
    strcpy(pairs[num_pairs].value, "Normal");
    num_pairs++;

    /* Contradictory: HeaderDigest with non-matching DataDigest */
    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "HeaderDigest");
    strcpy(pairs[num_pairs].value, "CRC32C");
    num_pairs++;

    pairs[num_pairs].key[0] = '\0';
    strcpy(pairs[num_pairs].key, "DataDigest");
    strcpy(pairs[num_pairs].value, "INVALID_DIGEST");
    num_pairs++;

    /* Allocate space for BHS (48) + data segment */
    data_segment = (uint8_t *)malloc(1024);
    if (!data_segment) {
        return NULL;
    }

    data_size = build_kv_segment(data_segment, 1024, pairs, num_pairs);
    if (data_size < 0) {
        free(data_segment);
        return NULL;
    }

    total_size = 48 + data_size;
    pdu = (uint8_t *)malloc(total_size);
    if (!pdu) {
        free(data_segment);
        return NULL;
    }

    /* Build BHS */
    memset(pdu, 0, 48);
    bhs = (iscsi_bhs_t *)pdu;

    bhs->opcode = ISCSI_OPCODE_LOGIN_REQUEST;
    bhs->flags = ISCSI_LOGIN_FLAG_TRANSIT | ISCSI_NSG_OPERATIONAL;
    bhs->version_max = 0x00;
    bhs->version_active = 0x00;
    encode_24bit(pdu + 1, data_size);
    encode_32bit(pdu + 16, 3);  /* Different Init Task Tag */
    encode_32bit(pdu + 24, 0);
    encode_32bit(pdu + 28, 0);

    /* Copy data segment */
    memcpy(pdu + 48, data_segment, data_size);
    free(data_segment);

    *pdu_size = total_size;
    return pdu;
}

/**
 * Send PDU to target and receive response
 */
uint8_t* send_pdu_and_recv_response(const char *host, int port,
                                     const uint8_t *pdu, size_t pdu_size,
                                     size_t *response_size) {
    struct sockaddr_in server_addr;
    struct hostent *server;
    int sock;
    uint8_t *response_buf;
    int bytes_recv;

    /* Create socket */
    sock = socket(AF_INET, SOCK_STREAM, 0);
    if (sock < 0) {
        return NULL;
    }

    /* Resolve hostname */
    server = gethostbyname(host);
    if (!server) {
        close(sock);
        return NULL;
    }

    /* Connect to server */
    memset(&server_addr, 0, sizeof(server_addr));
    server_addr.sin_family = AF_INET;
    server_addr.sin_port = htons(port);
    memcpy(&server_addr.sin_addr.s_addr, server->h_addr, server->h_length);

    if (connect(sock, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
        close(sock);
        return NULL;
    }

    /* Send PDU */
    if (send(sock, pdu, pdu_size, 0) < 0) {
        close(sock);
        return NULL;
    }

    /* Allocate response buffer (max iSCSI PDU + data) */
    response_buf = (uint8_t *)malloc(65536);
    if (!response_buf) {
        close(sock);
        return NULL;
    }

    /* Receive response */
    bytes_recv = recv(sock, response_buf, 65536, 0);
    close(sock);

    if (bytes_recv <= 0) {
        free(response_buf);
        return NULL;
    }

    *response_size = bytes_recv;
    return response_buf;
}

/**
 * Parse login response status
 * Returns: 0 if rejected, 1 if accepted, -1 on error
 */
int parse_login_response_status(const uint8_t *response, size_t response_size) {
    uint8_t status_class, status_detail;

    if (response_size < 48) {
        return -1;  /* Response too small */
    }

    /* Check opcode */
    if ((response[0] & 0x3F) != ISCSI_OPCODE_LOGIN_RESPONSE) {
        return -1;  /* Not a login response */
    }

    /* Status is at bytes 36-37 (big-endian) */
    status_class = response[36];
    status_detail = response[37];

    /* Status 0x00/0x00 = Success */
    if (status_class == 0x00 && status_detail == 0x00) {
        return 1;  /* Accepted */
    }

    /* Any other status = rejected/error */
    return 0;  /* Rejected */
}
