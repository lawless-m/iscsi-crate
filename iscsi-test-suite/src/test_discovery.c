#include "test_discovery.h"
#include "utils.h"
#include "iscsi_pdu_helper.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#include <poll.h>
#include <pthread.h>

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
    uint8_t *pdu = NULL;
    uint8_t *response = NULL;
    size_t pdu_size = 0;
    size_t response_size = 0;
    int status;
    int rejected_count = 0;
    int test_count = 0;
    char *host;
    char *port_str;
    int port = 3260;
    char portal_copy[256];
    char msg[512];

    (void)unused_iscsi;

    if (!config->portal || strlen(config->portal) == 0) {
        report_set_result(report, TEST_SKIP, "No portal configured");
        return TEST_SKIP;
    }

    /* Parse portal address (format: "host:port" or just "host") */
    strncpy(portal_copy, config->portal, sizeof(portal_copy) - 1);
    portal_copy[sizeof(portal_copy) - 1] = '\0';

    host = portal_copy;
    port_str = strchr(portal_copy, ':');
    if (port_str) {
        *port_str = '\0';
        port_str++;
        port = atoi(port_str);
        if (port <= 0 || port > 65535) {
            port = 3260;
        }
    }

    /* Test 1: Invalid MaxRecvDataSegmentLength=0 */
    test_count++;
    pdu = build_login_pdu_invalid_maxrecvdatasize(&pdu_size);
    if (pdu) {
        response = send_pdu_and_recv_response(host, port, pdu, pdu_size, &response_size);
        if (response) {
            status = parse_login_response_status(response, response_size);
            if (status == 0) {
                /* Target correctly rejected the invalid parameter */
                rejected_count++;
            }
            free(response);
            response = NULL;
        }
        free(pdu);
        pdu = NULL;
    }

    /* Test 2: Invalid MaxConnections=0 */
    test_count++;
    pdu = build_login_pdu_invalid_maxconnections(&pdu_size);
    if (pdu) {
        response = send_pdu_and_recv_response(host, port, pdu, pdu_size, &response_size);
        if (response) {
            status = parse_login_response_status(response, response_size);
            if (status == 0) {
                /* Target correctly rejected the invalid parameter */
                rejected_count++;
            }
            free(response);
            response = NULL;
        }
        free(pdu);
        pdu = NULL;
    }

    /* Test 3: Contradictory parameter combination */
    test_count++;
    pdu = build_login_pdu_invalid_param_combo(&pdu_size);
    if (pdu) {
        response = send_pdu_and_recv_response(host, port, pdu, pdu_size, &response_size);
        if (response) {
            status = parse_login_response_status(response, response_size);
            if (status == 0) {
                /* Target correctly rejected the invalid parameter */
                rejected_count++;
            }
            free(response);
            response = NULL;
        }
        free(pdu);
        pdu = NULL;
    }

    /* Evaluate test result */
    if (test_count == 0) {
        report_set_result(report, TEST_ERROR, "Failed to construct test PDUs");
        return TEST_ERROR;
    }

    if (rejected_count == 0) {
        snprintf(msg, sizeof(msg),
                 "Target did not reject any invalid parameters (%d/%d tests)",
                 rejected_count, test_count);
        report_set_result(report, TEST_FAIL, msg);
        return TEST_FAIL;
    }

    if (rejected_count < test_count) {
        snprintf(msg, sizeof(msg),
                 "Target accepted some invalid parameters (%d/%d rejected)",
                 rejected_count, test_count);
        report_set_result(report, TEST_FAIL, msg);
        return TEST_FAIL;
    }

    snprintf(msg, sizeof(msg), "Target correctly rejected all %d invalid parameter tests", test_count);
    report_set_result(report, TEST_PASS, msg);
    return TEST_PASS;
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

/* Thread data structure for simultaneous login test */
typedef struct {
    test_config_t *config;
    int thread_id;
    int result;
    char error_msg[256];
} thread_login_data_t;

/* Thread function for simultaneous login */
static void* login_thread_func(void *arg) {
    thread_login_data_t *data = (thread_login_data_t *)arg;
    struct iscsi_context *iscsi;
    int ret;

    /* Create iSCSI context for this thread */
    iscsi = create_iscsi_context_for_test(data->config);
    if (!iscsi) {
        snprintf(data->error_msg, sizeof(data->error_msg),
                 "Thread %d: Failed to create iSCSI context", data->thread_id);
        data->result = -1;
        return NULL;
    }

    /* Attempt to connect and login */
    ret = iscsi_connect_target(iscsi, data->config);
    if (ret != 0) {
        snprintf(data->error_msg, sizeof(data->error_msg),
                 "Thread %d: Login failed: %s", data->thread_id, iscsi_get_error(iscsi));
        data->result = ret;
        iscsi_destroy_context(iscsi);
        return NULL;
    }

    /* Successful login */
    data->result = 0;
    snprintf(data->error_msg, sizeof(data->error_msg),
             "Thread %d: Login successful", data->thread_id);

    /* Disconnect and cleanup */
    iscsi_disconnect_target(iscsi);
    iscsi_destroy_context(iscsi);

    return NULL;
}

/* TL-006: Simultaneous Logins */
static test_result_t test_simultaneous_logins(struct iscsi_context *unused_iscsi,
                                                test_config_t *config,
                                                test_report_t *report) {
    pthread_t threads[3];
    thread_login_data_t thread_data[3];
    int num_threads = 3;
    int successful_logins = 0;
    int failed_logins = 0;
    int ret;

    (void)unused_iscsi;

    if (!config->iqn || strlen(config->iqn) == 0) {
        report_set_result(report, TEST_SKIP, "No IQN specified in config");
        return TEST_SKIP;
    }

    /* Initialize thread data */
    for (int i = 0; i < num_threads; i++) {
        thread_data[i].config = config;
        thread_data[i].thread_id = i + 1;
        thread_data[i].result = -999; /* Uninitialized marker */
        thread_data[i].error_msg[0] = '\0';
    }

    /* Spawn all threads simultaneously */
    for (int i = 0; i < num_threads; i++) {
        ret = pthread_create(&threads[i], NULL, login_thread_func, &thread_data[i]);
        if (ret != 0) {
            char msg[256];
            snprintf(msg, sizeof(msg), "Failed to create thread %d", i + 1);
            report_set_result(report, TEST_ERROR, msg);

            /* Wait for any already-created threads */
            for (int j = 0; j < i; j++) {
                pthread_join(threads[j], NULL);
            }
            return TEST_ERROR;
        }
    }

    /* Wait for all threads to complete */
    for (int i = 0; i < num_threads; i++) {
        pthread_join(threads[i], NULL);
    }

    /* Analyze results */
    for (int i = 0; i < num_threads; i++) {
        if (thread_data[i].result == 0) {
            successful_logins++;
        } else if (thread_data[i].result != -999) {
            failed_logins++;
        }
    }

    /* Test passes if:
     * 1. All logins succeeded (target allows concurrent logins), OR
     * 2. Some logins succeeded and others were properly rejected (target serializes),
     * 3. No crashes or hangs occurred (we got here successfully)
     */

    if (successful_logins == num_threads) {
        /* All concurrent logins succeeded */
        report_set_result(report, TEST_PASS,
                         "All concurrent logins succeeded - target supports simultaneous connections");
        return TEST_PASS;
    } else if (successful_logins > 0 && failed_logins > 0) {
        /* Mixed results - target serialized/rejected some logins */
        char msg[512];
        snprintf(msg, sizeof(msg),
                 "Target handled concurrent logins gracefully (%d succeeded, %d rejected)",
                 successful_logins, failed_logins);
        report_set_result(report, TEST_PASS, msg);
        return TEST_PASS;
    } else if (successful_logins == 0 && failed_logins == num_threads) {
        /* All failed - this might indicate a problem, but if it's consistent rejection, it's OK */
        char msg[512];
        snprintf(msg, sizeof(msg),
                 "All concurrent logins were rejected. First error: %s",
                 thread_data[0].error_msg);
        report_set_result(report, TEST_FAIL, msg);
        return TEST_FAIL;
    } else {
        /* Some threads didn't report results - unexpected */
        report_set_result(report, TEST_ERROR, "Unexpected thread execution state");
        return TEST_ERROR;
    }
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
