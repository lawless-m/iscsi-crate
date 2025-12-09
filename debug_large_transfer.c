#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <iscsi/iscsi.h>
#include <iscsi/scsi-lowlevel.h>

int main() {
    struct iscsi_context *iscsi;
    struct iscsi_url *url;
    uint64_t num_blocks;
    uint32_t block_size;
    const int num_test_blocks = 256; // Same as TI-007
    uint8_t *write_buf, *read_buf;

    // Create context
    iscsi = iscsi_create_context("iqn.2025-12.local:initiator");
    if (!iscsi) {
        fprintf(stderr, "Failed to create context\n");
        return 1;
    }

    // Set session type
    if (iscsi_set_session_type(iscsi, ISCSI_SESSION_NORMAL) != 0) {
        fprintf(stderr, "Failed to set session type\n");
        return 1;
    }

    // Set target name
    if (iscsi_set_targetname(iscsi, "iqn.2025-12.local:storage.memory-disk") != 0) {
        fprintf(stderr, "Failed to set target name\n");
        return 1;
    }

    // Connect
    if (iscsi_connect_sync(iscsi, "127.0.0.1:3261") != 0) {
        fprintf(stderr, "Failed to connect: %s\n", iscsi_get_error(iscsi));
        return 1;
    }

    // Login
    if (iscsi_login_sync(iscsi) != 0) {
        fprintf(stderr, "Failed to login: %s\n", iscsi_get_error(iscsi));
        return 1;
    }

    printf("Connected successfully\n");

    // Get capacity
    struct scsi_task *task = iscsi_readcapacity10_sync(iscsi, 0, 0, 0);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        fprintf(stderr, "READ CAPACITY failed\n");
        return 1;
    }

    struct scsi_readcapacity10 *rc10 = scsi_datain_unmarshall(task);
    if (!rc10) {
        fprintf(stderr, "Failed to unmarshall READ CAPACITY\n");
        return 1;
    }

    block_size = rc10->block_size;
    num_blocks = rc10->lba + 1;
    scsi_free_scsi_task(task);
    task = NULL;

    printf("Capacity: %lu blocks of %u bytes\n", (unsigned long)num_blocks, block_size);

    // Allocate buffers
    size_t total_size = block_size * num_test_blocks;
    write_buf = malloc(total_size);
    read_buf = malloc(total_size);

    if (!write_buf || !read_buf) {
        fprintf(stderr, "Memory allocation failed\n");
        return 1;
    }

    // Fill write buffer with a pattern
    printf("Filling %zu bytes with pattern...\n", total_size);
    for (size_t i = 0; i < total_size; i++) {
        write_buf[i] = (uint8_t)(i & 0xFF);
    }

    printf("Writing %d blocks at LBA 5000...\n", num_test_blocks);

    // Write blocks
    task = iscsi_write10_sync(iscsi, 0, 5000, write_buf, total_size, block_size,
                             0, 0, 0, 0, 0);
    if (!task) {
        fprintf(stderr, "WRITE failed: %s\n", iscsi_get_error(iscsi));
        free(write_buf);
        free(read_buf);
        return 1;
    }

    if (task->status != SCSI_STATUS_GOOD) {
        fprintf(stderr, "WRITE status: %d\n", task->status);
        scsi_free_scsi_task(task);
        free(write_buf);
        free(read_buf);
        return 1;
    }

    scsi_free_scsi_task(task);
    printf("Write completed successfully\n");

    // Read back
    printf("Reading %d blocks from LBA 5000...\n", num_test_blocks);
    task = iscsi_read10_sync(iscsi, 0, 5000, total_size, block_size,
                            0, 0, 0, 0, 0);
    if (!task) {
        fprintf(stderr, "READ failed: %s\n", iscsi_get_error(iscsi));
        free(write_buf);
        free(read_buf);
        return 1;
    }

    if (task->status != SCSI_STATUS_GOOD) {
        fprintf(stderr, "READ status: %d\n", task->status);
        scsi_free_scsi_task(task);
        free(write_buf);
        free(read_buf);
        return 1;
    }

    printf("Read completed, data length: %d bytes\n", task->datain.size);

    if (task->datain.size != (int)total_size) {
        fprintf(stderr, "Data length mismatch: got %d, expected %zu\n",
                task->datain.size, total_size);
    }

    memcpy(read_buf, task->datain.data, task->datain.size);
    scsi_free_scsi_task(task);

    // Compare
    printf("Comparing data...\n");
    int mismatches = 0;
    for (size_t i = 0; i < total_size && i < (size_t)task->datain.size; i++) {
        if (write_buf[i] != read_buf[i]) {
            if (mismatches < 10) {
                printf("Mismatch at offset %zu: wrote 0x%02x, read 0x%02x\n",
                       i, write_buf[i], read_buf[i]);
            }
            mismatches++;
        }
    }

    if (mismatches > 0) {
        printf("FAILED: %d mismatches found\n", mismatches);
    } else {
        printf("SUCCESS: All data matches!\n");
    }

    // Show first 64 bytes of each
    printf("\nFirst 64 bytes written:\n");
    for (int i = 0; i < 64; i++) {
        printf("%02x ", write_buf[i]);
        if ((i + 1) % 16 == 0) printf("\n");
    }

    printf("\nFirst 64 bytes read:\n");
    for (int i = 0; i < 64; i++) {
        printf("%02x ", read_buf[i]);
        if ((i + 1) % 16 == 0) printf("\n");
    }

    free(write_buf);
    free(read_buf);
    iscsi_disconnect(iscsi);
    iscsi_destroy_context(iscsi);

    return (mismatches > 0) ? 1 : 0;
}
