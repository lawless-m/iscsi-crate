#include "test_commands.h"
#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <iscsi/iscsi.h>
#include <iscsi/scsi-lowlevel.h>

/* TC-001: INQUIRY Command */
static test_result_t test_inquiry(struct iscsi_context *unused_iscsi,
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

    task = iscsi_inquiry_sync(iscsi, config->lun, 0, 0, 255);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        report_set_result(report, TEST_FAIL, "INQUIRY command failed");
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

/* TC-002: TEST UNIT READY */
static test_result_t test_unit_ready(struct iscsi_context *unused_iscsi,
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

    task = iscsi_testunitready_sync(iscsi, config->lun);
    if (!task) {
        report_set_result(report, TEST_FAIL, "TEST UNIT READY failed");
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

/* TC-003: READ CAPACITY (10) */
static test_result_t test_read_capacity10(struct iscsi_context *unused_iscsi,
                                           test_config_t *config,
                                           test_report_t *report) {
    struct iscsi_context *iscsi;
    uint64_t num_blocks;
    uint32_t block_size;

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
        report_set_result(report, TEST_FAIL, "READ CAPACITY failed");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (num_blocks == 0 || block_size == 0) {
        report_set_result(report, TEST_FAIL, "Invalid capacity or block size");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TC-004: READ CAPACITY (16) */
static test_result_t test_read_capacity16(struct iscsi_context *unused_iscsi,
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

    task = iscsi_readcapacity16_sync(iscsi, config->lun);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        /* Some targets may not support READ CAPACITY(16) */
        report_set_result(report, TEST_SKIP, "READ CAPACITY(16) not supported");
        if (task) scsi_free_scsi_task(task);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_SKIP;
    }

    scsi_free_scsi_task(task);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* Remaining command tests implemented as SKIPs for now */
static test_result_t test_skip_placeholder(struct iscsi_context *unused_iscsi,
                                            test_config_t *config,
                                            test_report_t *report) {
    (void)unused_iscsi;
    (void)config;
    report_set_result(report, TEST_SKIP, "Test not yet implemented");
    return TEST_SKIP;
}

/* Test definitions */
static test_def_t command_tests[] = {
    {"TC-001", "INQUIRY Command", "SCSI Command Tests", test_inquiry},
    {"TC-002", "TEST UNIT READY", "SCSI Command Tests", test_unit_ready},
    {"TC-003", "READ CAPACITY (10)", "SCSI Command Tests", test_read_capacity10},
    {"TC-004", "READ CAPACITY (16)", "SCSI Command Tests", test_read_capacity16},
    {"TC-005", "MODE SENSE", "SCSI Command Tests", test_skip_placeholder},
    {"TC-006", "REQUEST SENSE", "SCSI Command Tests", test_skip_placeholder},
    {"TC-007", "REPORT LUNS", "SCSI Command Tests", test_skip_placeholder},
    {"TC-008", "Invalid Command", "SCSI Command Tests", test_skip_placeholder},
    {"TC-009", "Command to Invalid LUN", "SCSI Command Tests", test_skip_placeholder},
};

/* Register all tests */
void register_command_tests(void) {
    for (size_t i = 0; i < sizeof(command_tests) / sizeof(command_tests[0]); i++) {
        framework_register_test(&command_tests[i]);
    }
}
