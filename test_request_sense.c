/*
 * Test to verify REQUEST SENSE is being called
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

    printf("Test: Verify REQUEST SENSE retrieval\n");
    printf("=====================================\n\n");

    /* Create context */
    iscsi = iscsi_create_context("iscsi://127.0.0.1:3261/iqn.2025-12.local:storage.memory-disk/0");
    if (!iscsi) {
        fprintf(stderr, "ERROR: Could not create iSCSI context\n");
        return 1;
    }

    iscsi_set_targetname(iscsi, "iqn.2025-12.local:storage.memory-disk");
    iscsi_set_session_type(iscsi, ISCSI_SESSION_NORMAL);

    /* Connect */
    printf("[1] Connecting...\n");
    if (iscsi_full_connect_sync(iscsi, "127.0.0.1:3261", 0) != 0) {
        fprintf(stderr, "ERROR: Connection failed\n");
        iscsi_destroy_context(iscsi);
        return 1;
    }
    printf("    Connected\n\n");

    /* First, send an invalid command to generate an error */
    printf("[2] Sending invalid SCSI command (0xFF) to generate CHECK_CONDITION...\n");
    memset(cdb, 0, sizeof(cdb));
    cdb[0] = 0xFF;

    task = scsi_create_task(6, cdb, SCSI_XFER_NONE, 0);
    task = iscsi_scsi_command_sync(iscsi, 0, task, NULL);

    if (task) {
        printf("    Status: 0x%02x\n", task->status);
        printf("    Sense.key: 0x%02x\n", task->sense.key);
        printf("    Sense.error_type: 0x%02x\n", task->sense.error_type);
        scsi_free_scsi_task(task);
    }
    printf("\n");

    /* Now send REQUEST SENSE to retrieve the sense data */
    printf("[3] Sending REQUEST SENSE command...\n");
    task = scsi_cdb_requestsense_sync(iscsi, 0, 255);

    if (task) {
        printf("    Status: 0x%02x\n", task->status);
        printf("    Data length: %d bytes\n", task->datain.size);

        if (task->datain.size >= 3) {
            unsigned char *data = (unsigned char *)task->datain.data;
            printf("    Response code: 0x%02x\n", data[0]);
            printf("    Sense key (byte 2): 0x%02x\n", data[2]);
            printf("    ASC (byte 12): 0x%02x\n", data[12]);
        }
        printf("    Sense.key: 0x%02x\n", task->sense.key);
        printf("    Sense.error_type: 0x%02x\n", task->sense.error_type);

        scsi_free_scsi_task(task);
    }

    iscsi_disconnect(iscsi);
    iscsi_destroy_context(iscsi);

    return 0;
}
