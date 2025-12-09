/*
 * Debug test for TC-008 - Invalid Command sense key issue
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <iscsi/iscsi.h>
#include <iscsi/scsi-lowlevel.h>

int main() {
    struct iscsi_context *iscsi;
    struct scsi_task *task;
    unsigned char cdb[6];
    int ret;

    printf("Debug Test for TC-008 - Invalid Command\n");
    printf("========================================\n\n");

    /* Create context */
    printf("[1] Creating iSCSI context...\n");
    iscsi = iscsi_create_context("iqn.2025-12.test:debug");
    if (!iscsi) {
        fprintf(stderr, "ERROR: Could not create iSCSI context\n");
        return 1;
    }

    /* Set parameters */
    iscsi_set_targetname(iscsi, "iqn.2025-12.local:storage.memory-disk");
    iscsi_set_session_type(iscsi, ISCSI_SESSION_NORMAL);
    iscsi_set_header_digest(iscsi, ISCSI_HEADER_DIGEST_NONE);

    /* Connect */
    printf("[2] Connecting to 127.0.0.1:3261...\n");
    ret = iscsi_full_connect_sync(iscsi, "127.0.0.1:3261", 0);
    if (ret != 0) {
        fprintf(stderr, "ERROR: Connection failed: %s\n", iscsi_get_error(iscsi));
        iscsi_destroy_context(iscsi);
        return 1;
    }
    printf("    Connected successfully\n\n");

    /* Create a CDB with an invalid/unsupported opcode (0xFF) */
    printf("[3] Sending invalid SCSI command (opcode 0xFF)...\n");
    memset(cdb, 0, sizeof(cdb));
    cdb[0] = 0xFF;  /* Invalid opcode */

    /* Create task with the invalid CDB */
    task = scsi_create_task(6, cdb, SCSI_XFER_NONE, 0);
    if (!task) {
        fprintf(stderr, "ERROR: Failed to create task\n");
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return 1;
    }

    /* Execute the command */
    printf("    Executing command...\n");
    task = iscsi_scsi_command_sync(iscsi, 0, task, NULL);
    if (!task) {
        fprintf(stderr, "ERROR: Failed to execute command\n");
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return 1;
    }

    /* Print response details */
    printf("\n[4] Response Details:\n");
    printf("    Status: 0x%02x (%s)\n", task->status,
           task->status == SCSI_STATUS_GOOD ? "GOOD" :
           task->status == SCSI_STATUS_CHECK_CONDITION ? "CHECK CONDITION" :
           "OTHER");

    printf("    Sense Key: 0x%02x\n", task->sense.key);
    printf("    Sense ASCQ: 0x%02x\n", task->sense.ascq);
    printf("    Sense Error Type: 0x%02x\n", task->sense.error_type);
    printf("    Data-In size: %d\n", task->datain.size);

    printf("\n[5] Test Validation:\n");

    if (task->status != SCSI_STATUS_CHECK_CONDITION) {
        printf("    FAIL: Expected CHECK CONDITION status (0x%02x), got 0x%02x\n",
               SCSI_STATUS_CHECK_CONDITION, task->status);
    } else {
        printf("    PASS: Got CHECK CONDITION status\n");
    }

    printf("    Expected sense key: 0x%02x (SCSI_SENSE_ILLEGAL_REQUEST)\n", SCSI_SENSE_ILLEGAL_REQUEST);
    printf("    Actual sense key:   0x%02x\n", task->sense.key);

    if (task->sense.key != SCSI_SENSE_ILLEGAL_REQUEST) {
        printf("    FAIL: Sense key does not match\n");
    } else {
        printf("    PASS: Sense key matches SCSI_SENSE_ILLEGAL_REQUEST\n");
    }

    scsi_free_scsi_task(task);
    iscsi_disconnect(iscsi);
    iscsi_destroy_context(iscsi);

    return 0;
}
