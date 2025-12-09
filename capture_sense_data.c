/*
 * Capture and dump sense data from TC-008 test
 * Compile: gcc -o capture_sense_data capture_sense_data.c -liscsi
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <iscsi/iscsi.h>
#include <iscsi/scsi-lowlevel.h>

void hex_dump(const char *label, const unsigned char *data, size_t len) {
    printf("%s (%zu bytes):\n", label, len);
    for (size_t i = 0; i < len; i++) {
        printf("%02X ", data[i]);
        if ((i + 1) % 16 == 0) printf("\n");
    }
    if (len % 16 != 0) printf("\n");
}

int main(int argc, char **argv) {
    struct iscsi_context *iscsi;
    struct scsi_task *task;
    unsigned char cdb[6];
    const char *target = (argc > 1) ? argv[1] : "127.0.0.1";
    const char *iqn = (argc > 2) ? argv[2] : "iqn.2025-12.local:storage.memory-disk";

    printf("Capturing sense data from: %s (%s)\n\n", target, iqn);

    iscsi = iscsi_create_context("iqn.2025-12.test:capture");
    if (!iscsi) {
        fprintf(stderr, "Failed to create context\n");
        return 1;
    }

    iscsi_set_targetname(iscsi, iqn);
    iscsi_set_session_type(iscsi, ISCSI_SESSION_NORMAL);
    iscsi_set_header_digest(iscsi, ISCSI_HEADER_DIGEST_NONE);

    if (iscsi_full_connect_sync(iscsi, target, 3261) != 0) {
        fprintf(stderr, "Connect failed: %s\n", iscsi_get_error(iscsi));
        iscsi_destroy_context(iscsi);
        return 1;
    }

    // Send invalid command (opcode 0xFF)
    memset(cdb, 0, sizeof(cdb));
    cdb[0] = 0xFF;

    task = scsi_create_task(6, cdb, SCSI_XFER_NONE, 0);
    task = iscsi_scsi_command_sync(iscsi, 0, task, NULL);

    if (!task) {
        fprintf(stderr, "Command failed\n");
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return 1;
    }

    printf("SCSI Status: 0x%02X\n", task->status);
    printf("Sense Key: 0x%02X\n", task->sense.key);
    printf("ASC: 0x%02X\n", (task->sense.ascq >> 8) & 0xFF);
    printf("ASCQ: 0x%02X\n\n", task->sense.ascq & 0xFF);

    // Dump raw datain buffer which contains sense data
    if (task->datain.size > 0) {
        hex_dump("Raw sense data from SCSI Response",
                 (unsigned char*)task->datain.data,
                 task->datain.size);
    } else {
        printf("No sense data in datain buffer\n");
    }

    scsi_free_scsi_task(task);
    iscsi_disconnect(iscsi);
    iscsi_destroy_context(iscsi);

    return 0;
}
