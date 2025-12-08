#ifndef TEST_FRAMEWORK_H
#define TEST_FRAMEWORK_H

#include <iscsi/iscsi.h>
#include <stdbool.h>
#include <time.h>

/* Test result types */
typedef enum {
    TEST_PASS,
    TEST_FAIL,
    TEST_SKIP,
    TEST_ERROR
} test_result_t;

/* Test report structure */
typedef struct {
    const char *test_id;
    const char *test_name;
    const char *category;
    test_result_t result;
    char *message;
    double duration_ms;
} test_report_t;

/* Test configuration structure */
typedef struct {
    /* Target configuration */
    char *portal;
    char *iqn;
    int lun;

    /* Authentication */
    char *auth_method;
    char *username;
    char *password;
    char *mutual_username;
    char *mutual_password;

    /* Test parameters */
    int block_size;
    int large_transfer_blocks;
    int timeout;
    int stress_iterations;

    /* Options */
    int verbosity;
    bool stop_on_fail;
    bool generate_report;
} test_config_t;

/* Test function signature */
typedef test_result_t (*test_func_t)(struct iscsi_context *iscsi,
                                      test_config_t *config,
                                      test_report_t *report);

/* Test definition structure */
typedef struct {
    const char *test_id;
    const char *test_name;
    const char *category;
    test_func_t func;
} test_def_t;

/* Global test statistics */
typedef struct {
    int total;
    int passed;
    int failed;
    int skipped;
    int errors;
    double total_duration_ms;
} test_stats_t;

/* Test framework functions */
void framework_init(void);
void framework_cleanup(void);
void framework_register_test(test_def_t *test);
int framework_run_tests(test_config_t *config);
void framework_print_summary(test_stats_t *stats);
void framework_generate_report(test_config_t *config, test_stats_t *stats);

/* Helper functions for test reporting */
test_report_t* report_create(const char *test_id, const char *test_name, const char *category);
void report_set_result(test_report_t *report, test_result_t result, const char *message);
void report_free(test_report_t *report);

/* Test result helpers */
const char* result_to_string(test_result_t result);
const char* result_to_color(test_result_t result);

#endif /* TEST_FRAMEWORK_H */
