#include "test_io.h"
#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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

/* Placeholder for remaining tests */
static test_result_t test_skip_placeholder(struct iscsi_context *unused_iscsi,
                                            test_config_t *config,
                                            test_report_t *report) {
    (void)unused_iscsi;
    (void)config;
    report_set_result(report, TEST_SKIP, "Test not yet implemented");
    return TEST_SKIP;
}

/* Test definitions */
static test_def_t io_tests[] = {
    {"TI-001", "Single Block Read", "I/O Operation Tests", test_single_block_read},
    {"TI-002", "Single Block Write", "I/O Operation Tests", test_single_block_write},
    {"TI-003", "Multi-Block Sequential Read", "I/O Operation Tests", test_skip_placeholder},
    {"TI-004", "Multi-Block Sequential Write", "I/O Operation Tests", test_skip_placeholder},
    {"TI-005", "Random Access Reads", "I/O Operation Tests", test_skip_placeholder},
    {"TI-006", "Random Access Writes", "I/O Operation Tests", test_skip_placeholder},
    {"TI-007", "Large Transfer Read", "I/O Operation Tests", test_skip_placeholder},
    {"TI-008", "Large Transfer Write", "I/O Operation Tests", test_skip_placeholder},
    {"TI-009", "Zero-Length Transfer", "I/O Operation Tests", test_skip_placeholder},
    {"TI-010", "Maximum Transfer Size", "I/O Operation Tests", test_skip_placeholder},
    {"TI-011", "Beyond Maximum Transfer", "I/O Operation Tests", test_skip_placeholder},
    {"TI-012", "Unaligned Access", "I/O Operation Tests", test_skip_placeholder},
    {"TI-013", "Write-Read-Verify Pattern", "I/O Operation Tests", test_write_read_verify},
    {"TI-014", "Overwrite Test", "I/O Operation Tests", test_skip_placeholder},
};

/* Register all tests */
void register_io_tests(void) {
    for (size_t i = 0; i < sizeof(io_tests) / sizeof(io_tests[0]); i++) {
        framework_register_test(&io_tests[i]);
    }
}
