#include <stdio.h>
#include <string.h>
#include <iscsi/iscsi.h>
#include <iscsi/scsi-lowlevel.h>

int main(int argc, char *argv[]) {
    struct iscsi_context *iscsi;
    struct iscsi_url *url;
    struct scsi_task *task;
    unsigned char cdb[6];

    if (argc < 2) {
        fprintf(stderr, "Usage: %s <iscsi://...>\n", argv[0]);
        return 1;
    }

    // Parse URL
    url = iscsi_parse_full_url(NULL, argv[1]);
    if (!url) {
        fprintf(stderr, "Failed to parse URL\n");
        return 1;
    }

    // Create context
    iscsi = iscsi_create_context("iqn.2025-12.local:initiator");
    if (!iscsi) {
        fprintf(stderr, "Failed to create context\n");
        return 1;
    }

    // Connect
    if (iscsi_connect_sync(iscsi, url->portal) != 0) {
        fprintf(stderr, "Failed to connect: %s\n", iscsi_get_error(iscsi));
        iscsi_destroy_context(iscsi);
        return 1;
    }

    printf("Connected successfully\n");

    // Create invalid command
    memset(cdb, 0, sizeof(cdb));
    cdb[0] = 0xFF;  // Invalid opcode

    task = scsi_create_task(6, cdb, SCSI_XFER_NONE, 0);
    if (!task) {
        fprintf(stderr, "Failed to create task\n");
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return 1;
    }

    printf("Created task for invalid command\n");

    // Execute command
    task = iscsi_scsi_command_sync(iscsi, 0, task, NULL);
    if (!task) {
        fprintf(stderr, "Failed to execute command\n");
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return 1;
    }

    printf("Command executed\n");
    printf("Status: 0x%02x\n", task->status);
    printf("Sense key: 0x%02x (%s)\n", task->sense.key,
           task->sense.key == SCSI_SENSE_ILLEGAL_REQUEST ? "ILLEGAL_REQUEST" : "OTHER");
    printf("ASCQ: 0x%04x\n", task->sense.ascq);

    if (task->status == SCSI_STATUS_CHECK_CONDITION) {
        printf("✓ CHECK CONDITION status received\n");
    } else {
        printf("✗ Expected CHECK CONDITION, got 0x%02x\n", task->status);
    }

    if (task->sense.key == SCSI_SENSE_ILLEGAL_REQUEST) {
        printf("✓ ILLEGAL REQUEST sense key received\n");
    } else {
        printf("✗ Expected ILLEGAL REQUEST (0x05), got 0x%02x\n",
               task->sense.key);
    }

    scsi_free_scsi_task(task);
    iscsi_disconnect(iscsi);
    iscsi_destroy_context(iscsi);

    return 0;
}
