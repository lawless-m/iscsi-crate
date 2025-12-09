#include "test_io.h"
#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <iscsi/scsi-lowlevel.h>

/* TI-001: Single Block Read */
static test_result_t test_single_block_read(struct iscsi_context *unused_iscsi,
                                              test_config_t *config,
                                              test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_buf, *read_buf;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Get capacity */
    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    write_buf = malloc(block_size);
    read_buf = malloc(block_size);
    if (!write_buf || !read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Generate and write pattern */
    generate_pattern(write_buf, block_size, "sequential", 12345);
    if (scsi_write_blocks(iscsi, config->lun, 0, 1, block_size, write_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Write failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Read and verify */
    if (scsi_read_blocks(iscsi, config->lun, 0, 1, block_size, read_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Read failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (memcmp(write_buf, read_buf, block_size) != 0) {
        report_set_result(report, TEST_FAIL, "Data mismatch");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    free(write_buf);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-002: Single Block Write */
static test_result_t test_single_block_write(struct iscsi_context *unused_iscsi,
                                               test_config_t *config,
                                               test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_buf, *read_buf;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    write_buf = malloc(block_size);
    read_buf = malloc(block_size);
    if (!write_buf || !read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Write and verify */
    generate_pattern(write_buf, block_size, "alternating", 54321);
    if (scsi_write_blocks(iscsi, config->lun, 10, 1, block_size, write_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Write failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (scsi_read_blocks(iscsi, config->lun, 10, 1, block_size, read_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Read failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (memcmp(write_buf, read_buf, block_size) != 0) {
        report_set_result(report, TEST_FAIL, "Data mismatch after write");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    free(write_buf);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-013: Write-Read-Verify Pattern */
static test_result_t test_write_read_verify(struct iscsi_context *unused_iscsi,
                                              test_config_t *config,
                                              test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_buf, *read_buf;
    const char *patterns[] = {"zero", "ones", "alternating", "random"};
    int num_patterns = sizeof(patterns) / sizeof(patterns[0]);

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    write_buf = malloc(block_size);
    read_buf = malloc(block_size);
    if (!write_buf || !read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Test each pattern */
    for (int i = 0; i < num_patterns; i++) {
        generate_pattern(write_buf, block_size, patterns[i], 99999 + i);

        if (scsi_write_blocks(iscsi, config->lun, 100 + i, 1, block_size, write_buf) != 0) {
            char msg[256];
            snprintf(msg, sizeof(msg), "Write failed for pattern: %s", patterns[i]);
            report_set_result(report, TEST_FAIL, msg);
            free(write_buf);
            free(read_buf);
            iscsi_disconnect_target(iscsi);
            iscsi_destroy_context(iscsi);
            return TEST_FAIL;
        }

        if (scsi_read_blocks(iscsi, config->lun, 100 + i, 1, block_size, read_buf) != 0) {
            char msg[256];
            snprintf(msg, sizeof(msg), "Read failed for pattern: %s", patterns[i]);
            report_set_result(report, TEST_FAIL, msg);
            free(write_buf);
            free(read_buf);
            iscsi_disconnect_target(iscsi);
            iscsi_destroy_context(iscsi);
            return TEST_FAIL;
        }

        if (memcmp(write_buf, read_buf, block_size) != 0) {
            char msg[256];
            snprintf(msg, sizeof(msg), "Data mismatch for pattern: %s", patterns[i]);
            report_set_result(report, TEST_FAIL, msg);
            free(write_buf);
            free(read_buf);
            iscsi_disconnect_target(iscsi);
            iscsi_destroy_context(iscsi);
            return TEST_FAIL;
        }
    }

    free(write_buf);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-003: Multi-Block Sequential Read */
static test_result_t test_multiblock_sequential_read(struct iscsi_context *unused_iscsi,
                                                      test_config_t *config,
                                                      test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_buf, *read_buf;
    const int num_test_blocks = 16;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    size_t total_size = block_size * num_test_blocks;
    write_buf = malloc(total_size);
    read_buf = malloc(total_size);
    if (!write_buf || !read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Write sequential pattern */
    generate_pattern(write_buf, total_size, "sequential", 11111);
    if (scsi_write_blocks(iscsi, config->lun, 200, num_test_blocks, block_size, write_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Multi-block write failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Read and verify */
    if (scsi_read_blocks(iscsi, config->lun, 200, num_test_blocks, block_size, read_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Multi-block read failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (memcmp(write_buf, read_buf, total_size) != 0) {
        report_set_result(report, TEST_FAIL, "Multi-block data mismatch");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    free(write_buf);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-004: Multi-Block Sequential Write */
static test_result_t test_multiblock_sequential_write(struct iscsi_context *unused_iscsi,
                                                       test_config_t *config,
                                                       test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_buf, *read_buf;
    const int num_test_blocks = 32;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    size_t total_size = block_size * num_test_blocks;
    write_buf = malloc(total_size);
    read_buf = malloc(total_size);
    if (!write_buf || !read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Write alternating pattern */
    generate_pattern(write_buf, total_size, "alternating", 22222);
    if (scsi_write_blocks(iscsi, config->lun, 300, num_test_blocks, block_size, write_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Multi-block sequential write failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Verify */
    if (scsi_read_blocks(iscsi, config->lun, 300, num_test_blocks, block_size, read_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Verification read failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (memcmp(write_buf, read_buf, total_size) != 0) {
        report_set_result(report, TEST_FAIL, "Data mismatch after multi-block write");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    free(write_buf);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-005: Random Access Reads */
static test_result_t test_random_access_reads(struct iscsi_context *unused_iscsi,
                                               test_config_t *config,
                                               test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_buf, *read_buf;
    uint64_t test_lbas[] = {0, 10, 100, 500, 1000};
    int num_lbas = sizeof(test_lbas) / sizeof(test_lbas[0]);

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    write_buf = malloc(block_size);
    read_buf = malloc(block_size);
    if (!write_buf || !read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Write and verify at random LBAs */
    for (int i = 0; i < num_lbas; i++) {
        if (test_lbas[i] >= num_blocks) continue;

        generate_pattern(write_buf, block_size, "sequential", 33333 + i);

        if (scsi_write_blocks(iscsi, config->lun, test_lbas[i], 1, block_size, write_buf) != 0) {
            char msg[256];
            snprintf(msg, sizeof(msg), "Random write failed at LBA %lu", (unsigned long)test_lbas[i]);
            report_set_result(report, TEST_FAIL, msg);
            free(write_buf);
            free(read_buf);
            iscsi_disconnect_target(iscsi);
            iscsi_destroy_context(iscsi);
            return TEST_FAIL;
        }

        if (scsi_read_blocks(iscsi, config->lun, test_lbas[i], 1, block_size, read_buf) != 0) {
            char msg[256];
            snprintf(msg, sizeof(msg), "Random read failed at LBA %lu", (unsigned long)test_lbas[i]);
            report_set_result(report, TEST_FAIL, msg);
            free(write_buf);
            free(read_buf);
            iscsi_disconnect_target(iscsi);
            iscsi_destroy_context(iscsi);
            return TEST_FAIL;
        }

        if (memcmp(write_buf, read_buf, block_size) != 0) {
            char msg[256];
            snprintf(msg, sizeof(msg), "Data mismatch at LBA %lu", (unsigned long)test_lbas[i]);
            report_set_result(report, TEST_FAIL, msg);
            free(write_buf);
            free(read_buf);
            iscsi_disconnect_target(iscsi);
            iscsi_destroy_context(iscsi);
            return TEST_FAIL;
        }
    }

    free(write_buf);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-006: Random Access Writes */
static test_result_t test_random_access_writes(struct iscsi_context *unused_iscsi,
                                                test_config_t *config,
                                                test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_bufs[5], *read_buf;
    uint64_t test_lbas[] = {1500, 750, 2000, 250, 1250};
    int num_lbas = sizeof(test_lbas) / sizeof(test_lbas[0]);

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Allocate buffers */
    read_buf = malloc(block_size);
    for (int i = 0; i < num_lbas; i++) {
        write_bufs[i] = malloc(block_size);
        if (!write_bufs[i]) {
            report_set_result(report, TEST_ERROR, "Memory allocation failed");
            for (int j = 0; j < i; j++) free(write_bufs[j]);
            free(read_buf);
            iscsi_disconnect_target(iscsi);
            iscsi_destroy_context(iscsi);
            return TEST_ERROR;
        }
    }

    if (!read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        for (int i = 0; i < num_lbas; i++) free(write_bufs[i]);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Write different patterns to non-sequential LBAs */
    for (int i = 0; i < num_lbas; i++) {
        if (test_lbas[i] >= num_blocks) continue;

        generate_pattern(write_bufs[i], block_size, "random", 44444 + i);

        if (scsi_write_blocks(iscsi, config->lun, test_lbas[i], 1, block_size, write_bufs[i]) != 0) {
            char msg[256];
            snprintf(msg, sizeof(msg), "Random write failed at LBA %lu", (unsigned long)test_lbas[i]);
            report_set_result(report, TEST_FAIL, msg);
            for (int j = 0; j < num_lbas; j++) free(write_bufs[j]);
            free(read_buf);
            iscsi_disconnect_target(iscsi);
            iscsi_destroy_context(iscsi);
            return TEST_FAIL;
        }
    }

    /* Verify all writes */
    for (int i = 0; i < num_lbas; i++) {
        if (test_lbas[i] >= num_blocks) continue;

        if (scsi_read_blocks(iscsi, config->lun, test_lbas[i], 1, block_size, read_buf) != 0) {
            char msg[256];
            snprintf(msg, sizeof(msg), "Verification read failed at LBA %lu", (unsigned long)test_lbas[i]);
            report_set_result(report, TEST_FAIL, msg);
            for (int j = 0; j < num_lbas; j++) free(write_bufs[j]);
            free(read_buf);
            iscsi_disconnect_target(iscsi);
            iscsi_destroy_context(iscsi);
            return TEST_FAIL;
        }

        if (memcmp(write_bufs[i], read_buf, block_size) != 0) {
            char msg[256];
            snprintf(msg, sizeof(msg), "Data mismatch at LBA %lu", (unsigned long)test_lbas[i]);
            report_set_result(report, TEST_FAIL, msg);
            for (int j = 0; j < num_lbas; j++) free(write_bufs[j]);
            free(read_buf);
            iscsi_disconnect_target(iscsi);
            iscsi_destroy_context(iscsi);
            return TEST_FAIL;
        }
    }

    for (int i = 0; i < num_lbas; i++) free(write_bufs[i]);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-007: Large Transfer Read */
static test_result_t test_large_transfer_read(struct iscsi_context *unused_iscsi,
                                               test_config_t *config,
                                               test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_buf, *read_buf;
    const int num_test_blocks = 256; /* 256 blocks = potentially 128KB+ */

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (num_blocks < num_test_blocks + 1000) {
        report_set_result(report, TEST_SKIP, "Insufficient capacity for large transfer test");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_SKIP;
    }

    size_t total_size = block_size * num_test_blocks;
    write_buf = malloc(total_size);
    read_buf = malloc(total_size);
    if (!write_buf || !read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Write large transfer */
    generate_pattern(write_buf, total_size, "random", 55555);
    if (scsi_write_blocks(iscsi, config->lun, 5000, num_test_blocks, block_size, write_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Large transfer write failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Read and verify */
    if (scsi_read_blocks(iscsi, config->lun, 5000, num_test_blocks, block_size, read_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Large transfer read failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (memcmp(write_buf, read_buf, total_size) != 0) {
        report_set_result(report, TEST_FAIL, "Large transfer data mismatch");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    free(write_buf);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-008: Large Transfer Write */
static test_result_t test_large_transfer_write(struct iscsi_context *unused_iscsi,
                                                test_config_t *config,
                                                test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_buf, *read_buf;
    const int num_test_blocks = 512; /* 512 blocks = potentially 256KB+ */

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (num_blocks < num_test_blocks + 6000) {
        report_set_result(report, TEST_SKIP, "Insufficient capacity for large write test");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_SKIP;
    }

    size_t total_size = block_size * num_test_blocks;
    write_buf = malloc(total_size);
    read_buf = malloc(total_size);
    if (!write_buf || !read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Write large pattern */
    generate_pattern(write_buf, total_size, "sequential", 66666);
    if (scsi_write_blocks(iscsi, config->lun, 6000, num_test_blocks, block_size, write_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Large write failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Verify */
    if (scsi_read_blocks(iscsi, config->lun, 6000, num_test_blocks, block_size, read_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Verification read failed after large write");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (memcmp(write_buf, read_buf, total_size) != 0) {
        report_set_result(report, TEST_FAIL, "Data mismatch after large write");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    free(write_buf);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-009: Zero-Length Transfer */
static test_result_t test_zero_length_transfer(struct iscsi_context *unused_iscsi,
                                                test_config_t *config,
                                                test_report_t *report) {
    struct iscsi_context *iscsi;
    struct scsi_task *task;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Try READ(10) with transfer length 0 - should be no-op per SCSI spec */
    task = iscsi_read10_sync(iscsi, config->lun, 0, 0, 512, 0, 0, 0, 0, 0);

    /* Zero-length transfer should succeed (it's a no-op) */
    if (!task || task->status != SCSI_STATUS_GOOD) {
        report_set_result(report, TEST_FAIL, "Zero-length transfer rejected");
        if (task) scsi_free_scsi_task(task);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    scsi_free_scsi_task(task);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-010: Maximum Transfer Size */
static test_result_t test_maximum_transfer_size(struct iscsi_context *unused_iscsi,
                                                 test_config_t *config,
                                                 test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_buf, *read_buf;
    /* Test at typical MaxBurstLength boundary: 256KB */
    /* With 512-byte blocks, this is 512 blocks */
    const int num_test_blocks = 512;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (num_blocks < num_test_blocks + 10000) {
        report_set_result(report, TEST_SKIP, "Insufficient capacity for max burst test");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_SKIP;
    }

    size_t total_size = block_size * num_test_blocks;
    write_buf = malloc(total_size);
    read_buf = malloc(total_size);
    if (!write_buf || !read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Write at typical MaxBurstLength boundary */
    generate_pattern(write_buf, total_size, "random", 10101);
    if (scsi_write_blocks(iscsi, config->lun, 10000, num_test_blocks, block_size, write_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Write at MaxBurstLength boundary failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Read and verify */
    if (scsi_read_blocks(iscsi, config->lun, 10000, num_test_blocks, block_size, read_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Read at MaxBurstLength boundary failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (memcmp(write_buf, read_buf, total_size) != 0) {
        report_set_result(report, TEST_FAIL, "Data mismatch at MaxBurstLength boundary");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    free(write_buf);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-011: Beyond Maximum Transfer */
static test_result_t test_beyond_maximum_transfer(struct iscsi_context *unused_iscsi,
                                                   test_config_t *config,
                                                   test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_buf, *read_buf;
    /* Test well beyond typical MaxBurstLength: 2MB */
    /* With 512-byte blocks, this is 4096 blocks */
    /* This should trigger multi-sequence handling in libiscsi */
    const int num_test_blocks = 4096;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (num_blocks < num_test_blocks + 15000) {
        report_set_result(report, TEST_SKIP, "Insufficient capacity for beyond-max-burst test");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_SKIP;
    }

    size_t total_size = block_size * num_test_blocks;
    write_buf = malloc(total_size);
    read_buf = malloc(total_size);
    if (!write_buf || !read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Write beyond MaxBurstLength - libiscsi should split into multiple sequences */
    generate_pattern(write_buf, total_size, "sequential", 20202);
    if (scsi_write_blocks(iscsi, config->lun, 15000, num_test_blocks, block_size, write_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Write beyond MaxBurstLength failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Read and verify - should also be split into multiple sequences */
    if (scsi_read_blocks(iscsi, config->lun, 15000, num_test_blocks, block_size, read_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Read beyond MaxBurstLength failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (memcmp(write_buf, read_buf, total_size) != 0) {
        report_set_result(report, TEST_FAIL, "Data mismatch for beyond-MaxBurstLength transfer");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    free(write_buf);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-012: Unaligned Access */
static test_result_t test_unaligned_access(struct iscsi_context *unused_iscsi,
                                            test_config_t *config,
                                            test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_buf, *read_buf;
    /* Test odd number of blocks at odd LBA */
    const int num_test_blocks = 7;
    const uint64_t start_lba = 1357;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (num_blocks < start_lba + num_test_blocks) {
        report_set_result(report, TEST_SKIP, "Insufficient capacity for unaligned access test");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_SKIP;
    }

    size_t total_size = block_size * num_test_blocks;
    write_buf = malloc(total_size);
    read_buf = malloc(total_size);
    if (!write_buf || !read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    generate_pattern(write_buf, total_size, "alternating", 77777);
    if (scsi_write_blocks(iscsi, config->lun, start_lba, num_test_blocks, block_size, write_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Unaligned write failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (scsi_read_blocks(iscsi, config->lun, start_lba, num_test_blocks, block_size, read_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Unaligned read failed");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (memcmp(write_buf, read_buf, total_size) != 0) {
        report_set_result(report, TEST_FAIL, "Unaligned access data mismatch");
        free(write_buf);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    free(write_buf);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TI-014: Overwrite Test */
static test_result_t test_overwrite(struct iscsi_context *unused_iscsi,
                                     test_config_t *config,
                                     test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;
    uint8_t *write_buf1, *write_buf2, *read_buf;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi || iscsi_connect_target(iscsi, config) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect");
        if (iscsi) iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    if (scsi_read_capacity(iscsi, config->lun, &num_blocks, &block_size) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to get capacity");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    write_buf1 = malloc(block_size);
    write_buf2 = malloc(block_size);
    read_buf = malloc(block_size);
    if (!write_buf1 || !write_buf2 || !read_buf) {
        report_set_result(report, TEST_ERROR, "Memory allocation failed");
        free(write_buf1);
        free(write_buf2);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Write first pattern */
    generate_pattern(write_buf1, block_size, "ones", 88888);
    if (scsi_write_blocks(iscsi, config->lun, 7000, 1, block_size, write_buf1) != 0) {
        report_set_result(report, TEST_FAIL, "Initial write failed");
        free(write_buf1);
        free(write_buf2);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Overwrite with different pattern */
    generate_pattern(write_buf2, block_size, "zero", 99999);
    if (scsi_write_blocks(iscsi, config->lun, 7000, 1, block_size, write_buf2) != 0) {
        report_set_result(report, TEST_FAIL, "Overwrite failed");
        free(write_buf1);
        free(write_buf2);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Read and verify we get the second pattern, not the first */
    if (scsi_read_blocks(iscsi, config->lun, 7000, 1, block_size, read_buf) != 0) {
        report_set_result(report, TEST_FAIL, "Read after overwrite failed");
        free(write_buf1);
        free(write_buf2);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (memcmp(write_buf2, read_buf, block_size) != 0) {
        report_set_result(report, TEST_FAIL, "Overwrite did not replace data");
        free(write_buf1);
        free(write_buf2);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Make sure it's NOT the first pattern */
    if (memcmp(write_buf1, read_buf, block_size) == 0) {
        report_set_result(report, TEST_FAIL, "Overwrite failed - original data still present");
        free(write_buf1);
        free(write_buf2);
        free(read_buf);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    free(write_buf1);
    free(write_buf2);
    free(read_buf);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* Test definitions */
static test_def_t io_tests[] = {
    {"TI-001", "Single Block Read", "I/O Operation Tests", test_single_block_read},
    {"TI-002", "Single Block Write", "I/O Operation Tests", test_single_block_write},
    {"TI-003", "Multi-Block Sequential Read", "I/O Operation Tests", test_multiblock_sequential_read},
    {"TI-004", "Multi-Block Sequential Write", "I/O Operation Tests", test_multiblock_sequential_write},
    {"TI-005", "Random Access Reads", "I/O Operation Tests", test_random_access_reads},
    {"TI-006", "Random Access Writes", "I/O Operation Tests", test_random_access_writes},
    {"TI-007", "Large Transfer Read", "I/O Operation Tests", test_large_transfer_read},
    {"TI-008", "Large Transfer Write", "I/O Operation Tests", test_large_transfer_write},
    {"TI-009", "Zero-Length Transfer", "I/O Operation Tests", test_zero_length_transfer},
    {"TI-010", "Maximum Transfer Size", "I/O Operation Tests", test_maximum_transfer_size},
    {"TI-011", "Beyond Maximum Transfer", "I/O Operation Tests", test_beyond_maximum_transfer},
    {"TI-012", "Unaligned Access", "I/O Operation Tests", test_unaligned_access},
    {"TI-013", "Write-Read-Verify Pattern", "I/O Operation Tests", test_write_read_verify},
    {"TI-014", "Overwrite Test", "I/O Operation Tests", test_overwrite},
};

/* Register all tests */
void register_io_tests(void) {
    for (size_t i = 0; i < sizeof(io_tests) / sizeof(io_tests[0]); i++) {
        framework_register_test(&io_tests[i]);
    }
}
