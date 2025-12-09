/*
 * Debug test for large transfer read/write
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <iscsi/iscsi.h>
#include <iscsi/scsi-lowlevel.h>

void generate_pattern(uint8_t *buffer, size_t size, uint32_t seed) {
    srand(seed);
    for (size_t i = 0; i < size; i++) {
        buffer[i] = (uint8_t)(rand() & 0xFF);
    }
}

int main(int argc, char *argv[]) {
    struct iscsi_context *iscsi;
    struct iscsi_url *url;
    struct scsi_task *task;

    const char *target_url = "iscsi://127.0.0.1:3262/iqn.2025-12.local:storage.memory-disk/0";
    const int lun = 0;
    const int num_blocks = 256;
    const uint32_t block_size = 512;
    const size_t total_size = num_blocks * block_size;

    printf("Test: Large transfer read/write\n");
    printf("Blocks: %d, Block size: %d, Total: %zu bytes\n", num_blocks, block_size, total_size);

    // Parse URL
    url = iscsi_parse_full_url(NULL, target_url);
    if (!url) {
        fprintf(stderr, "Failed to parse URL\n");
        return 1;
    }

    // Create context
    iscsi = iscsi_create_context("iqn.2024-12.com.test:initiator");
    if (!iscsi) {
        fprintf(stderr, "Failed to create iSCSI context\n");
        return 1;
    }

    iscsi_set_targetname(iscsi, url->target);
    iscsi_set_session_type(iscsi, ISCSI_SESSION_NORMAL);
    iscsi_set_header_digest(iscsi, ISCSI_HEADER_DIGEST_NONE);

    // Connect
    printf("Connecting to %s...\n", url->portal);
    if (iscsi_full_connect_sync(iscsi, url->portal, url->lun) != 0) {
        fprintf(stderr, "Failed to connect: %s\n", iscsi_get_error(iscsi));
        return 1;
    }
    printf("Connected!\n");

    // Allocate buffers
    uint8_t *write_buf = malloc(total_size);
    uint8_t *read_buf = malloc(total_size);
    if (!write_buf || !read_buf) {
        fprintf(stderr, "Memory allocation failed\n");
        return 1;
    }

    // Generate pattern
    generate_pattern(write_buf, total_size, 55555);
    printf("Generated write pattern (first 16 bytes): ");
    for (int i = 0; i < 16; i++) printf("%02x ", write_buf[i]);
    printf("\n");

    // Write
    printf("Writing %d blocks at LBA 5000...\n", num_blocks);
    task = iscsi_write10_sync(iscsi, lun, 5000, write_buf, total_size, block_size, 0, 0, 0, 0, 0);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        fprintf(stderr, "Write failed: %s\n", task ? "status not good" : "no task");
        if (task) {
            fprintf(stderr, "Task status: %d, residual: %d\n", task->status, task->residual);
            scsi_free_scsi_task(task);
        }
        return 1;
    }
    scsi_free_scsi_task(task);
    printf("Write complete\n");

    // Read back
    printf("Reading %d blocks at LBA 5000...\n", num_blocks);
    memset(read_buf, 0, total_size);
    task = iscsi_read10_sync(iscsi, lun, 5000, total_size, block_size, 0, 0, 0, 0, 0);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        fprintf(stderr, "Read failed: %s\n", task ? "status not good" : "no task");
        if (task) {
            fprintf(stderr, "Task status: %d, residual: %d\n", task->status, task->residual);
            scsi_free_scsi_task(task);
        }
        return 1;
    }

    printf("Read complete, datain size: %zu\n", task->datain.size);

    if (task->datain.size != total_size) {
        fprintf(stderr, "ERROR: Expected %zu bytes, got %zu\n", total_size, task->datain.size);
    }

    memcpy(read_buf, task->datain.data, task->datain.size);
    scsi_free_scsi_task(task);

    // Compare
    printf("Read pattern (first 16 bytes): ");
    for (int i = 0; i < 16; i++) printf("%02x ", read_buf[i]);
    printf("\n");

    int mismatch_count = 0;
    int first_mismatch = -1;
    for (size_t i = 0; i < total_size; i++) {
        if (write_buf[i] != read_buf[i]) {
            if (first_mismatch < 0) first_mismatch = i;
            mismatch_count++;
            if (mismatch_count <= 10) {
                printf("Mismatch at offset %zu: expected 0x%02x, got 0x%02x\n",
                       i, write_buf[i], read_buf[i]);
            }
        }
    }

    if (mismatch_count > 0) {
        printf("\nFAILED: %d bytes mismatch (first at offset %d)\n", mismatch_count, first_mismatch);

        // Show blocks around the mismatch
        if (first_mismatch >= 0) {
            int block = first_mismatch / block_size;
            printf("First mismatch is in block %d (offset within block: %d)\n",
                   block, first_mismatch % block_size);
        }
    } else {
        printf("\nSUCCESS: All %zu bytes match!\n", total_size);
    }

    // Cleanup
    iscsi_logout_sync(iscsi);
    iscsi_disconnect(iscsi);
    iscsi_destroy_context(iscsi);
    iscsi_destroy_url(url);
    free(write_buf);
    free(read_buf);

    return mismatch_count > 0 ? 1 : 0;
}
