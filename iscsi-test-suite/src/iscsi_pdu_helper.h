#ifndef ISCSI_PDU_HELPER_H
#define ISCSI_PDU_HELPER_H

#include <stdint.h>
#include <sys/socket.h>

/* RFC 3720 iSCSI PDU structures */

/* Basic Header Segment (BHS) - 48 bytes */
typedef struct {
    uint8_t opcode;           /* Opcode: login=0x03 */
    uint8_t flags;            /* Immediate, Transit, Continue, etc. */
    uint8_t version_max;      /* Maximum version supported */
    uint8_t version_active;   /* Active version */
    uint32_t length;          /* Total AHS + Data Segment length */
    uint64_t lun;             /* Logical Unit Number */
    uint64_t init_task_tag;   /* Initiator Task Tag */
    uint32_t cmd_sn;          /* Command Sequence Number */
    uint32_t exp_stat_sn;     /* Expected Status Sequence Number */
    uint32_t reserved[4];     /* Reserved */
} iscsi_bhs_t;

/* Key-Value pair for login negotiation */
typedef struct {
    char key[256];
    char value[256];
} iscsi_kv_pair_t;

/**
 * Build a malformed iSCSI Login PDU with invalid parameters
 *
 * Returns allocated buffer with PDU, caller must free()
 * Sets pdu_size to total PDU size
 */
uint8_t* build_login_pdu_invalid_maxrecvdatasize(size_t *pdu_size);
uint8_t* build_login_pdu_invalid_maxconnections(size_t *pdu_size);
uint8_t* build_login_pdu_invalid_param_combo(size_t *pdu_size);

/**
 * Send PDU to target and receive response
 * Returns allocated buffer with response PDU, caller must free()
 * Returns NULL on error
 */
uint8_t* send_pdu_and_recv_response(const char *host, int port,
                                     const uint8_t *pdu, size_t pdu_size,
                                     size_t *response_size);

/**
 * Parse login response status
 * Returns 0 if login was rejected, 1 if accepted, -1 on parse error
 */
int parse_login_response_status(const uint8_t *response, size_t response_size);

#endif /* ISCSI_PDU_HELPER_H */
