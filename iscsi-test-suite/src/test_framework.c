#include "test_framework.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>
#include <time.h>

/* Maximum number of tests */
#define MAX_TESTS 256

/* Global test registry */
static test_def_t *test_registry[MAX_TESTS];
static int test_count = 0;

/* Global test reports */
static test_report_t **test_reports = NULL;
static int report_count = 0;

/* Initialize test framework */
void framework_init(void) {
    test_count = 0;
    report_count = 0;
    test_reports = NULL;
}

/* Cleanup test framework */
void framework_cleanup(void) {
    if (test_reports) {
        for (int i = 0; i < report_count; i++) {
            report_free(test_reports[i]);
        }
        free(test_reports);
        test_reports = NULL;
    }
    report_count = 0;
}

/* Register a test */
void framework_register_test(test_def_t *test) {
    if (test_count < MAX_TESTS) {
        test_registry[test_count++] = test;
    }
}

/* Get current time in milliseconds */
static double get_time_ms(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (tv.tv_sec * 1000.0) + (tv.tv_usec / 1000.0);
}

/* Create a test report */
test_report_t* report_create(const char *test_id, const char *test_name, const char *category) {
    test_report_t *report = calloc(1, sizeof(test_report_t));
    if (!report) {
        return NULL;
    }

    report->test_id = test_id;
    report->test_name = test_name;
    report->category = category;
    report->result = TEST_ERROR;
    report->message = NULL;
    report->duration_ms = 0.0;

    return report;
}

/* Set test result */
void report_set_result(test_report_t *report, test_result_t result, const char *message) {
    report->result = result;
    if (report->message) {
        free(report->message);
    }
    report->message = message ? strdup(message) : NULL;
}

/* Free a test report */
void report_free(test_report_t *report) {
    if (report) {
        if (report->message) {
            free(report->message);
        }
        free(report);
    }
}

/* Convert result to string */
const char* result_to_string(test_result_t result) {
    switch (result) {
        case TEST_PASS:  return "PASS";
        case TEST_FAIL:  return "FAIL";
        case TEST_SKIP:  return "SKIP";
        case TEST_ERROR: return "ERROR";
        default:         return "UNKNOWN";
    }
}

/* Convert result to color code */
const char* result_to_color(test_result_t result) {
    switch (result) {
        case TEST_PASS:  return "\033[32m";  // Green
        case TEST_FAIL:  return "\033[31m";  // Red
        case TEST_SKIP:  return "\033[33m";  // Yellow
        case TEST_ERROR: return "\033[35m";  // Magenta
        default:         return "\033[0m";   // Reset
    }
}

/* Print test result */
static void print_test_result(test_report_t *report, int verbosity) {
    const char *color = result_to_color(report->result);
    const char *reset = "\033[0m";

    printf("  %s: %-40s [%s%s%s]  (%.3fs)\n",
           report->test_id,
           report->test_name,
           color,
           result_to_string(report->result),
           reset,
           report->duration_ms / 1000.0);

    if (report->message && (verbosity > 0 || report->result == TEST_FAIL || report->result == TEST_ERROR)) {
        printf("    └─ %s\n", report->message);
    }
}

/* Run all registered tests */
int framework_run_tests(test_config_t *config) {
    struct iscsi_context *iscsi = NULL;
    const char *current_category = NULL;
    test_stats_t stats = {0};

    /* Allocate reports array */
    test_reports = calloc(test_count, sizeof(test_report_t *));
    if (!test_reports) {
        fprintf(stderr, "Failed to allocate memory for test reports\n");
        return -1;
    }

    printf("\niSCSI Target Test Suite\n");
    printf("=======================\n");
    printf("Target: %s\n", config->portal);
    if (config->iqn && strlen(config->iqn) > 0) {
        printf("IQN: %s\n", config->iqn);
    }
    printf("LUN: %d\n\n", config->lun);

    /* Run each test */
    for (int i = 0; i < test_count; i++) {
        test_def_t *test = test_registry[i];

        /* Print category header if changed */
        if (!current_category || strcmp(current_category, test->category) != 0) {
            current_category = test->category;
            printf("\n[%s]\n", current_category);
        }

        /* Create test report */
        test_report_t *report = report_create(test->test_id, test->test_name, test->category);
        if (!report) {
            fprintf(stderr, "Failed to create test report\n");
            continue;
        }

        /* Run the test */
        double start_time = get_time_ms();
        test_result_t result = test->func(iscsi, config, report);
        double end_time = get_time_ms();

        report->duration_ms = end_time - start_time;
        if (report->result == TEST_ERROR) {
            report->result = result;
        }

        /* Update statistics */
        stats.total++;
        stats.total_duration_ms += report->duration_ms;
        switch (report->result) {
            case TEST_PASS:  stats.passed++; break;
            case TEST_FAIL:  stats.failed++; break;
            case TEST_SKIP:  stats.skipped++; break;
            case TEST_ERROR: stats.errors++; break;
        }

        /* Print result */
        print_test_result(report, config->verbosity);

        /* Store report */
        test_reports[report_count++] = report;

        /* Stop on failure if requested */
        if (config->stop_on_fail && report->result == TEST_FAIL) {
            printf("\nStopping on first failure (stop_on_fail=true)\n");
            break;
        }
    }

    /* Print summary */
    framework_print_summary(&stats);

    /* Generate report if requested */
    if (config->generate_report) {
        framework_generate_report(config, &stats);
    }

    /* Return non-zero if any failures */
    return (stats.failed > 0 || stats.errors > 0) ? 1 : 0;
}

/* Print test summary */
void framework_print_summary(test_stats_t *stats) {
    printf("\n=======================\n");
    printf("Results: %d passed, %d failed, %d skipped, %d errors\n",
           stats->passed, stats->failed, stats->skipped, stats->errors);
    printf("Duration: %.1f seconds\n", stats->total_duration_ms / 1000.0);
}

/* Generate detailed report */
void framework_generate_report(test_config_t *config, test_stats_t *stats) {
    char filename[256];
    time_t now = time(NULL);
    struct tm *t = localtime(&now);

    snprintf(filename, sizeof(filename),
             "reports/test_report_%04d%02d%02d_%02d%02d%02d.txt",
             t->tm_year + 1900, t->tm_mon + 1, t->tm_mday,
             t->tm_hour, t->tm_min, t->tm_sec);

    FILE *f = fopen(filename, "w");
    if (!f) {
        fprintf(stderr, "Failed to create report file: %s\n", filename);
        return;
    }

    fprintf(f, "iSCSI Target Test Suite - Detailed Report\n");
    fprintf(f, "==========================================\n");
    fprintf(f, "Date: %04d-%02d-%02d %02d:%02d:%02d\n",
            t->tm_year + 1900, t->tm_mon + 1, t->tm_mday,
            t->tm_hour, t->tm_min, t->tm_sec);
    fprintf(f, "Target: %s\n", config->portal);
    if (config->iqn && strlen(config->iqn) > 0) {
        fprintf(f, "IQN: %s\n", config->iqn);
    }
    fprintf(f, "LUN: %d\n\n", config->lun);

    fprintf(f, "Test Results:\n");
    fprintf(f, "-------------\n\n");

    const char *current_category = NULL;
    for (int i = 0; i < report_count; i++) {
        test_report_t *report = test_reports[i];

        if (!current_category || strcmp(current_category, report->category) != 0) {
            current_category = report->category;
            fprintf(f, "\n[%s]\n", current_category);
        }

        fprintf(f, "  %s: %s - %s (%.3fs)\n",
                report->test_id,
                report->test_name,
                result_to_string(report->result),
                report->duration_ms / 1000.0);

        if (report->message) {
            fprintf(f, "    Message: %s\n", report->message);
        }
    }

    fprintf(f, "\n\nSummary:\n");
    fprintf(f, "--------\n");
    fprintf(f, "Total:   %d\n", stats->total);
    fprintf(f, "Passed:  %d\n", stats->passed);
    fprintf(f, "Failed:  %d\n", stats->failed);
    fprintf(f, "Skipped: %d\n", stats->skipped);
    fprintf(f, "Errors:  %d\n", stats->errors);
    fprintf(f, "Duration: %.1f seconds\n", stats->total_duration_ms / 1000.0);

    fclose(f);
    printf("\nDetailed report saved to: %s\n", filename);
}
