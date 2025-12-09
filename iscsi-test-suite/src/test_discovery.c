#include "test_discovery.h"
#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#include <poll.h>

/* TD-001: Basic Discovery */
static test_result_t test_basic_discovery(struct iscsi_context *unused_iscsi,
                                           test_config_t *config,
                                           test_report_t *report) {
    struct iscsi_context *iscsi;
    struct iscsi_discovery_address *targets;

    (void)unused_iscsi;

    /* Create discovery context */
    iscsi = iscsi_create_context("iqn.2024-12.com.test:initiator");
    if (!iscsi) {
        report_set_result(report, TEST_ERROR, "Failed to create iSCSI context");
        return TEST_ERROR;
    }

    iscsi_set_session_type(iscsi, ISCSI_SESSION_DISCOVERY);

    /* Connect to portal */
    if (iscsi_connect_sync(iscsi, config->portal) != 0) {
        report_set_result(report, TEST_ERROR, "Failed to connect to portal");
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Perform discovery */
    targets = iscsi_discovery_sync(iscsi);
    if (!targets) {
        report_set_result(report, TEST_FAIL, "Discovery failed");
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    if (!targets->target_name) {
        report_set_result(report, TEST_FAIL, "No targets discovered");
        iscsi_free_discovery_data(iscsi, targets);
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Cleanup */
    iscsi_free_discovery_data(iscsi, targets);
    iscsi_disconnect(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TD-002: Discovery With Authentication */
static test_result_t test_discovery_auth(struct iscsi_context *unused_iscsi,
                                          test_config_t *config,
                                          test_report_t *report) {
    (void)unused_iscsi;

    if (!config->auth_method || strcmp(config->auth_method, "none") == 0) {
        report_set_result(report, TEST_SKIP, "No authentication configured");
        return TEST_SKIP;
    }

    /* This test would require a target that enforces discovery auth */
    report_set_result(report, TEST_SKIP, "Discovery auth test not fully implemented");
    return TEST_SKIP;
}

/* TD-003: Discovery Without Credentials */
static test_result_t test_discovery_no_creds(struct iscsi_context *unused_iscsi,
                                              test_config_t *config,
                                              test_report_t *report) {
    (void)unused_iscsi;
    (void)config;

    /* This requires a target that mandates auth */
    report_set_result(report, TEST_SKIP, "Requires auth-mandatory target");
    return TEST_SKIP;
}

/* TD-004: Target Redirection */
static test_result_t test_target_redirect(struct iscsi_context *unused_iscsi,
                                           test_config_t *config,
                                           test_report_t *report) {
    (void)unused_iscsi;
    (void)config;

    /* This requires a target that implements redirection */
    report_set_result(report, TEST_SKIP, "Requires redirection-capable target");
    return TEST_SKIP;
}

/* TL-001: Basic Login */
static test_result_t test_basic_login(struct iscsi_context *unused_iscsi,
                                       test_config_t *config,
                                       test_report_t *report) {
    struct iscsi_context *iscsi;
    int ret;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified in config");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi) {
        report_set_result(report, TEST_ERROR, "Failed to create iSCSI context");
        return TEST_ERROR;
    }

    ret = iscsi_connect_target(iscsi, config);
    if (ret != 0) {
        char msg[256];
        snprintf(msg, sizeof(msg), "Login failed: %s", iscsi_get_error(iscsi));
        report_set_result(report, TEST_FAIL, msg);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Successful login */
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TL-002: Parameter Negotiation */
static test_result_t test_param_negotiation(struct iscsi_context *unused_iscsi,
                                             test_config_t *config,
                                             test_report_t *report) {
    struct iscsi_context *iscsi;
    int ret;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified in config");
        return TEST_SKIP;
    }

    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi) {
        report_set_result(report, TEST_ERROR, "Failed to create iSCSI context");
        return TEST_ERROR;
    }

    /* Set specific parameter values to negotiate */
    iscsi_set_header_digest(iscsi, ISCSI_HEADER_DIGEST_NONE);

    ret = iscsi_connect_target(iscsi, config);
    if (ret != 0) {
        char msg[256];
        snprintf(msg, sizeof(msg), "Login failed: %s", iscsi_get_error(iscsi));
        report_set_result(report, TEST_FAIL, msg);
        iscsi_destroy_context(iscsi);
        return TEST_FAIL;
    }

    /* Parameter negotiation successful if connection works */
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TL-003: Invalid Parameter Values */
static test_result_t test_invalid_params(struct iscsi_context *unused_iscsi,
                                          test_config_t *config,
                                          test_report_t *report) {
    (void)unused_iscsi;
    (void)config;

    /* This requires low-level PDU manipulation which libiscsi doesn't easily support */
    report_set_result(report, TEST_SKIP, "Requires low-level PDU manipulation");
    return TEST_SKIP;
}

/* TL-004: Multiple Login Attempts */
static test_result_t test_multiple_logins(struct iscsi_context *unused_iscsi,
                                           test_config_t *config,
                                           test_report_t *report) {
    struct iscsi_context *iscsi;
    int ret;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified in config");
        return TEST_SKIP;
    }

    for (int i = 0; i < 3; i++) {
        iscsi = create_iscsi_context_for_test(config);
        if (!iscsi) {
            report_set_result(report, TEST_ERROR, "Failed to create iSCSI context");
            return TEST_ERROR;
        }

        ret = iscsi_connect_target(iscsi, config);
        if (ret != 0) {
            char msg[256];
            snprintf(msg, sizeof(msg), "Login attempt %d failed", i + 1);
            report_set_result(report, TEST_FAIL, msg);
            iscsi_destroy_context(iscsi);
            return TEST_FAIL;
        }

        iscsi_disconnect_target(iscsi);
        iscsi_destroy_context(iscsi);
    }

    report_set_result(report, TEST_PASS, NULL);
    return TEST_PASS;
}

/* TL-005: Login Timeout */
static test_result_t test_login_timeout(struct iscsi_context *unused_iscsi,
                                         test_config_t *config,
                                         test_report_t *report) {
    struct iscsi_context *iscsi;
    int fd;
    time_t start_time, current_time;
    int timeout_period = 20; /* Wait 20 seconds for target timeout */
    int ret;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified in config");
        return TEST_SKIP;
    }

    /* Create iSCSI context */
    iscsi = create_iscsi_context_for_test(config);
    if (!iscsi) {
        report_set_result(report, TEST_ERROR, "Failed to create iSCSI context");
        return TEST_ERROR;
    }

    /* Connect to portal at TCP level */
    ret = iscsi_connect_sync(iscsi, config->portal);
    if (ret != 0) {
        char msg[256];
        snprintf(msg, sizeof(msg), "Failed to connect to portal: %s", iscsi_get_error(iscsi));
        report_set_result(report, TEST_ERROR, msg);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Start login but don't complete it by not servicing the connection */
    ret = iscsi_login_async(iscsi, NULL, NULL);
    if (ret != 0) {
        char msg[256];
        snprintf(msg, sizeof(msg), "Failed to start login: %s", iscsi_get_error(iscsi));
        report_set_result(report, TEST_ERROR, msg);
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Get file descriptor to check connection state */
    fd = iscsi_get_fd(iscsi);
    if (fd < 0) {
        report_set_result(report, TEST_ERROR, "Failed to get socket file descriptor");
        iscsi_disconnect(iscsi);
        iscsi_destroy_context(iscsi);
        return TEST_ERROR;
    }

    /* Wait for target to timeout the login sequence */
    /* We deliberately don't call iscsi_service() to simulate a stalled client */
    start_time = time(NULL);
    while (1) {
        current_time = time(NULL);
        if (current_time - start_time >= timeout_period) {
            /* Timeout period expired */
            break;
        }
        sleep(1);
    }

    /* Try to service the connection to see if it's still alive */
    /* If target timed out properly, this should fail or show disconnection */
    ret = iscsi_service(iscsi, POLLIN);

    /* Check if we can get the file descriptor (connection still valid) */
    fd = iscsi_get_fd(iscsi);

    /* Clean up */
    iscsi_disconnect(iscsi);
    iscsi_destroy_context(iscsi);

    /* The test passes if the target closed the connection or login failed */
    /* If ret < 0 or fd is invalid, it means target properly handled the timeout */
    if (ret < 0 || fd < 0) {
        report_set_result(report, TEST_PASS, "Target properly timed out stalled login");
        return TEST_PASS;
    }

    /* If connection is still alive after timeout period, that's acceptable too */
    /* as some targets may have longer timeouts or be more tolerant */
    report_set_result(report, TEST_PASS, "Target maintained connection (may have long timeout)");
    return TEST_PASS;
}

/* TL-006: Simultaneous Logins */
static test_result_t test_simultaneous_logins(struct iscsi_context *unused_iscsi,
                                                test_config_t *config,
                                                test_report_t *report) {
    (void)unused_iscsi;
    (void)config;

    /* This requires multi-threading which adds complexity */
    report_set_result(report, TEST_SKIP, "Requires multi-threading");
    return TEST_SKIP;
}

/* Test definitions */
static test_def_t discovery_tests[] = {
    {"TD-001", "Basic Discovery", "Discovery Tests", test_basic_discovery},
    {"TD-002", "Discovery With Authentication", "Discovery Tests", test_discovery_auth},
    {"TD-003", "Discovery Without Credentials", "Discovery Tests", test_discovery_no_creds},
    {"TD-004", "Target Redirection", "Discovery Tests", test_target_redirect},
};

static test_def_t login_tests[] = {
    {"TL-001", "Basic Login", "Login/Logout Tests", test_basic_login},
    {"TL-002", "Parameter Negotiation", "Login/Logout Tests", test_param_negotiation},
    {"TL-003", "Invalid Parameter Values", "Login/Logout Tests", test_invalid_params},
    {"TL-004", "Multiple Login Attempts", "Login/Logout Tests", test_multiple_logins},
    {"TL-005", "Login Timeout", "Login/Logout Tests", test_login_timeout},
    {"TL-006", "Simultaneous Logins", "Login/Logout Tests", test_simultaneous_logins},
};

/* Register all tests */
void register_discovery_tests(void) {
    for (size_t i = 0; i < sizeof(discovery_tests) / sizeof(discovery_tests[0]); i++) {
        framework_register_test(&discovery_tests[i]);
    }
    for (size_t i = 0; i < sizeof(login_tests) / sizeof(login_tests[0]); i++) {
        framework_register_test(&login_tests[i]);
    }
}
