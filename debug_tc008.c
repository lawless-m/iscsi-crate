#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <iscsi/iscsi.h>
#include <iscsi/scsi-lowlevel.h>

struct iscsi_context *create_iscsi_context(const char *target_iqn) {
    struct iscsi_context *iscsi = iscsi_create_context(NULL);
    if (!iscsi) {
        return NULL;
    }
    iscsi_set_initiator_name(iscsi, "iqn.2025-12.local:test");
    iscsi_set_session_type(iscsi, ISCSI_SESSION_NORMAL);
    iscsi_set_targetname(iscsi, target_iqn);
    iscsi_set_session_initiator_username(iscsi, NULL);
    iscsi_set_session_initiator_password(iscsi, NULL);
    return iscsi;
}

int main() {
    struct iscsi_context *iscsi;
    struct scsi_task *task;
    unsigned char cdb[6];

    // Create iSCSI context
    iscsi = create_iscsi_context("iqn.2025-12.local:storage.memory-disk");
    if (!iscsi) {
        printf("Failed to create iSCSI context\n");
        return 1;
    }

    // Connect to target
    printf("Connecting to 127.0.0.1:3261\n");
    if (iscsi_connect_sync(iscsi, "127.0.0.1") != 0) {
        printf("Failed to connect: %s\n", iscsi_get_error(iscsi));
        iscsi_destroy_context(iscsi);
        return 1;
    }

    printf("Connected. Now sending invalid command (opcode 0xFF)...\n");

    // Create a CDB with an invalid opcode (0xFF)
    memset(cdb, 0, sizeof(cdb));
    cdb[0] = 0xFF;  // Invalid opcode

    // Create task with the invalid CDB
    task = scsi_create_task(6, cdb, SCSI_XFER_NONE, 0);
    if (!task) {
        printf("Failed to create task\n");
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return 1;
    }

    printf("Sending command with CDB[0]=0x%02x (LUN=0)...\n", cdb[0]);

    // Execute the command
    task = iscsi_scsi_command_sync(iscsi, 0, task, NULL);
    if (!task) {
        printf("Failed to execute command\n");
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return 1;
    }

    printf("\nResponse received:\n");
    printf("  Status: 0x%02x (CHECK_CONDITION=0x02)\n", task->status);
    printf("  Sense key: 0x%02x (ILLEGAL_REQUEST=0x05)\n", task->sense.key);
    printf("  ASCQ: 0x%04x\n", task->sense.ascq);

    if (task->status != SCSI_STATUS_CHECK_CONDITION) {
        printf("\nERROR: Expected CHECK CONDITION (0x02), got 0x%02x\n", task->status);
        scsi_free_scsi_task(task);
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return 1;
    }

    if (task->sense.key != SCSI_SENSE_ILLEGAL_REQUEST) {
        printf("\nERROR: Expected ILLEGAL REQUEST (0x05), got 0x%02x\n", task->sense.key);
        scsi_free_scsi_task(task);
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return 1;
    }

    printf("\nSUCCESS: Got expected CHECK CONDITION with ILLEGAL REQUEST sense key\n");

    scsi_free_scsi_task(task);
    iscsi_disconnect(iscsi);
    iscsi_destroy_context(iscsi);

    return 0;
}
