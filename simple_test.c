/*
 * Simple iSCSI target test - completes quickly with clear output
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <iscsi/iscsi.h>
#include <iscsi/scsi-lowlevel.h>

int main(int argc, char *argv[]) {
    struct iscsi_context *iscsi;
    struct iscsi_url *iscsi_url;
    const char *url;
    int ret;
    int tests_passed = 0;
    int tests_failed = 0;

    if (argc < 2) {
        fprintf(stderr, "Usage: %s iscsi://portal/target/lun\n", argv[0]);
        fprintf(stderr, "Example: %s iscsi://127.0.0.1:3261/iqn.2025-12.local:storage.memory-disk/0\n", argv[0]);
        return 2;
    }

    url = argv[1];
    printf("Simple iSCSI Target Test\n");
    fflush(stdout);
    printf("========================\n");
    fflush(stdout);
    printf("Target: %s\n\n", url);
    fflush(stdout);

    /* Parse URL */
    printf("Parsing URL...\n");
    fflush(stdout);
    iscsi_url = iscsi_parse_full_url(NULL, url);
    if (!iscsi_url) {
        fprintf(stderr, "ERROR: Invalid URL\n");
        return 2;
    }
    printf("URL parsed successfully\n");
    fflush(stdout);

    /* Create context */
    printf("[1/5] Creating iSCSI context...\n");
    fflush(stdout);
    iscsi = iscsi_create_context("iqn.2025-12.test:simple-tester");
    if (!iscsi) {
        fprintf(stderr, "  FAIL: Could not create iSCSI context\n");
        iscsi_destroy_url(iscsi_url);
        return 1;
    }
    printf("  PASS\n");
    fflush(stdout);
    tests_passed++;

    /* Set parameters */
    printf("Setting target name to: %s\n", iscsi_url->target);
    fflush(stdout);
    iscsi_set_targetname(iscsi, iscsi_url->target);
    iscsi_set_session_type(iscsi, ISCSI_SESSION_NORMAL);
    iscsi_set_header_digest(iscsi, ISCSI_HEADER_DIGEST_NONE);

    /* Connect */
    printf("[2/5] Connecting to target at %s...\n", iscsi_url->portal);
    fflush(stdout);
    ret = iscsi_full_connect_sync(iscsi, iscsi_url->portal, iscsi_url->lun);
    printf("Connect returned: %d\n", ret);
    fflush(stdout);
    if (ret != 0) {
        fprintf(stderr, "  FAIL: Connection failed: %s\n", iscsi_get_error(iscsi));
        iscsi_destroy_context(iscsi);
        iscsi_destroy_url(iscsi_url);
        tests_failed++;
        goto summary;
    }
    printf("  PASS: Connected successfully\n");
    tests_passed++;

    /* Test INQUIRY */
    printf("[3/5] Testing INQUIRY command...\n");
    struct scsi_task *task = iscsi_inquiry_sync(iscsi, iscsi_url->lun, 0, 0, 255);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        fprintf(stderr, "  FAIL: INQUIRY failed\n");
        if (task) scsi_free_scsi_task(task);
        tests_failed++;
    } else {
        printf("  PASS: INQUIRY successful\n");
        scsi_free_scsi_task(task);
        tests_passed++;
    }

    /* Test READ CAPACITY */
    printf("[4/5] Testing READ CAPACITY command...\n");
    task = iscsi_readcapacity10_sync(iscsi, iscsi_url->lun, 0, 0);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        fprintf(stderr, "  FAIL: READ CAPACITY failed\n");
        if (task) scsi_free_scsi_task(task);
        tests_failed++;
    } else {
        if (task->datain.size >= 8) {
            unsigned char *buf = task->datain.data;
            uint32_t last_lba = (buf[0] << 24) | (buf[1] << 16) | (buf[2] << 8) | buf[3];
            uint32_t blk_size = (buf[4] << 24) | (buf[5] << 16) | (buf[6] << 8) | buf[7];
            printf("  PASS: Capacity = %u blocks x %u bytes\n", last_lba + 1, blk_size);
            tests_passed++;
        } else {
            fprintf(stderr, "  FAIL: Invalid response size\n");
            tests_failed++;
        }
        scsi_free_scsi_task(task);
    }

    /* Test Read/Write */
    printf("[5/5] Testing READ/WRITE operations...\n");
    unsigned char write_buf[512];
    unsigned char read_buf[512];
    memset(write_buf, 0xAA, sizeof(write_buf));

    task = iscsi_write10_sync(iscsi, iscsi_url->lun, 0, write_buf, sizeof(write_buf), 512, 0, 0, 0, 0, 0);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        fprintf(stderr, "  FAIL: WRITE failed\n");
        if (task) scsi_free_scsi_task(task);
        tests_failed++;
    } else {
        scsi_free_scsi_task(task);

        task = iscsi_read10_sync(iscsi, iscsi_url->lun, 0, sizeof(read_buf), 512, 0, 0, 0, 0, 0);
        if (!task || task->status != SCSI_STATUS_GOOD) {
            fprintf(stderr, "  FAIL: READ failed\n");
            if (task) scsi_free_scsi_task(task);
            tests_failed++;
        } else {
            memcpy(read_buf, task->datain.data, sizeof(read_buf));
            scsi_free_scsi_task(task);

            if (memcmp(write_buf, read_buf, sizeof(write_buf)) == 0) {
                printf("  PASS: Data integrity verified\n");
                tests_passed++;
            } else {
                fprintf(stderr, "  FAIL: Data mismatch\n");
                tests_failed++;
            }
        }
    }

    /* Disconnect */
    iscsi_logout_sync(iscsi);
    iscsi_disconnect(iscsi);

summary:
    /* Cleanup */
    iscsi_destroy_context(iscsi);
    iscsi_destroy_url(iscsi_url);

    /* Summary */
    printf("\n========================\n");
    printf("Summary: %d passed, %d failed\n", tests_passed, tests_failed);
    printf("========================\n");

    return (tests_failed > 0) ? 1 : 0;
}
