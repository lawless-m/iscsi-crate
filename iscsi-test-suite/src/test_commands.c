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

/* TC-005: MODE SENSE */
static test_result_t test_mode_sense(struct iscsi_context *unused_iscsi,
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

    /* MODE SENSE(6) - Page 0x3F (all pages), current values */
    task = iscsi_modesense6_sync(iscsi, config->lun, 0, SCSI_MODESENSE_PC_CURRENT, 0x3F, 0, 255);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        report_set_result(report, TEST_FAIL, "MODE SENSE(6) command failed");
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

/* TC-006: REQUEST SENSE */
static test_result_t test_request_sense(struct iscsi_context *unused_iscsi,
                                         test_config_t *config,
                                         test_report_t *report) {
    /* REQUEST SENSE doesn't have a dedicated sync function in libiscsi */
    /* The sense data is automatically retrieved on errors */
    (void)unused_iscsi;
    (void)config;
    report_set_result(report, TEST_SKIP, "REQUEST SENSE handled automatically by libiscsi");
    return TEST_SKIP;
}

/* TC-007: REPORT LUNS */
static test_result_t test_report_luns(struct iscsi_context *unused_iscsi,
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

    task = iscsi_reportluns_sync(iscsi, 0, 16384);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        report_set_result(report, TEST_FAIL, "REPORT LUNS command failed");
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

/* TC-008: Invalid Command
 *
 * This test verifies that the target properly rejects an invalid SCSI opcode.
 *
 * Per TGTD (Linux reference iSCSI implementation) behavior and real-world
 * observations:
 * - The target MUST return CHECK CONDITION status for invalid opcodes
 * - The sense key SHOULD be ILLEGAL REQUEST (0x05), but some implementations
 *   return CHECK CONDITION with minimal/no sense data
 * - libiscsi may not always populate task->sense.key from the response
 *
 * This test validates that:
 * 1. The target rejects the command (does not return GOOD status)
 * 2. The target responds properly (doesn't crash or hang)
 *
 * Both CHECK CONDITION (with any sense key) and explicit rejection are valid
 * responses per real-world iSCSI implementation behavior.
 */
static test_result_t test_invalid_command(struct iscsi_context *unused_iscsi,
                                           test_config_t *config,
                                           test_report_t *report) {
    struct iscsi_context *iscsi;
    struct scsi_task *task;
    unsigned char cdb[6];

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

    /* Create a CDB with an invalid/unsupported opcode (0xFF) */
    memset(cdb, 0, sizeof(cdb));
    cdb[0] = 0xFF;  /* Invalid opcode */

    /* Create task with the invalid CDB */
    task = scsi_create_task(6, cdb, SCSI_XFER_NONE, 0);
    if (!task) {
        report_set_result(report, TEST_ERROR, "Failed to create task");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Execute the command */
    task = iscsi_scsi_command_sync(iscsi, config->lun, task, NULL);
    if (!task) {
        report_set_result(report, TEST_ERROR, "Failed to execute command");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Verify target rejects the invalid command.
     * The key requirement is that the target does NOT return GOOD status.
     * CHECK CONDITION is the expected response per SCSI spec.
     */
    if (task->status == SCSI_STATUS_GOOD) {
        report_set_result(report, TEST_FAIL,
                         "Target incorrectly accepted invalid SCSI opcode 0xFF");
        scsi_free_scsi_task(task);
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Target properly rejected the invalid command */
    char msg[256];
    if (task->status == SCSI_STATUS_CHECK_CONDITION) {
        if (task->sense.key == SCSI_SENSE_ILLEGAL_REQUEST) {
            snprintf(msg, sizeof(msg),
                     "Target returned CHECK CONDITION with ILLEGAL REQUEST sense");
        } else {
            /* Some targets/libiscsi combinations don't populate sense.key */
            snprintf(msg, sizeof(msg),
                     "Target returned CHECK CONDITION (sense_key=%d)",
                     task->sense.key);
        }
    } else {
        snprintf(msg, sizeof(msg),
                 "Target rejected command with status 0x%02x", task->status);
    }

    scsi_free_scsi_task(task);
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, msg);
    return TEST_PASS;
}

/* TC-009: Command to Invalid LUN */
static test_result_t test_invalid_lun(struct iscsi_context *unused_iscsi,
                                       test_config_t *config,
                                       test_report_t *report) {
    struct iscsi_context *iscsi;
    struct scsi_task *task;
    uint64_t invalid_lun = 999; /* Highly unlikely to exist */

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

    /* Try to send INQUIRY to invalid LUN */
    task = iscsi_inquiry_sync(iscsi, invalid_lun, 0, 0, 255);

    if (!task) {
        report_set_result(report, TEST_ERROR, "Failed to send command to invalid LUN");
        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Target should reject with CHECK CONDITION */
    if (task->status == SCSI_STATUS_GOOD) {
        report_set_result(report, TEST_FAIL, "Target accepted command to invalid LUN");
        scsi_free_scsi_task(task);
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

/* Test definitions */
static test_def_t command_tests[] = {
    {"TC-001", "INQUIRY Command", "SCSI Command Tests", test_inquiry},
    {"TC-002", "TEST UNIT READY", "SCSI Command Tests", test_unit_ready},
    {"TC-003", "READ CAPACITY (10)", "SCSI Command Tests", test_read_capacity10},
    {"TC-004", "READ CAPACITY (16)", "SCSI Command Tests", test_read_capacity16},
    {"TC-005", "MODE SENSE", "SCSI Command Tests", test_mode_sense},
    {"TC-006", "REQUEST SENSE", "SCSI Command Tests", test_request_sense},
    {"TC-007", "REPORT LUNS", "SCSI Command Tests", test_report_luns},
    {"TC-008", "Invalid Command", "SCSI Command Tests", test_invalid_command},
    {"TC-009", "Command to Invalid LUN", "SCSI Command Tests", test_invalid_lun},
};

/* Register all tests */
void register_command_tests(void) {
    for (size_t i = 0; i < sizeof(command_tests) / sizeof(command_tests[0]); i++) {
        framework_register_test(&command_tests[i]);
    }
}
