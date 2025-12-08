#include "test_framework.h"
#include "test_discovery.h"
#include "test_commands.h"
#include "test_io.h"
#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <getopt.h>

static void print_usage(const char *progname) {
    printf("Usage: %s [options] <config_file>\n", progname);
    printf("\nOptions:\n");
    printf("  -v, --verbose      Verbose output\n");
    printf("  -q, --quiet        Quiet mode (only show failures)\n");
    printf("  -f, --fail-fast    Stop on first failure\n");
    printf("  -c, --category CAT Run specific test category\n");
    printf("  -h, --help         Show this help message\n");
    printf("\nAvailable categories:\n");
    printf("  discovery          Discovery and login tests\n");
    printf("  commands           SCSI command tests\n");
    printf("  io                 I/O operation tests\n");
    printf("  all                All tests (default)\n");
}

int main(int argc, char *argv[]) {
    test_config_t config;
    int ret;
    const char *config_file = NULL;
    const char *category = "all";
    int opt;

    static struct option long_options[] = {
        {"verbose",   no_argument,       0, 'v'},
        {"quiet",     no_argument,       0, 'q'},
        {"fail-fast", no_argument,       0, 'f'},
        {"category",  required_argument, 0, 'c'},
        {"help",      no_argument,       0, 'h'},
        {0, 0, 0, 0}
    };

    /* Parse command line options */
    while ((opt = getopt_long(argc, argv, "vqfc:h", long_options, NULL)) != -1) {
        switch (opt) {
            case 'v':
                /* Verbose mode - set after config loaded */
                break;
            case 'q':
                /* Quiet mode - set after config loaded */
                break;
            case 'f':
                /* Fail-fast mode - set after config loaded */
                break;
            case 'c':
                category = optarg;
                break;
            case 'h':
                print_usage(argv[0]);
                return 0;
            default:
                print_usage(argv[0]);
                return 2;
        }
    }

    /* Get config file */
    if (optind >= argc) {
        fprintf(stderr, "Error: Config file required\n\n");
        print_usage(argv[0]);
        return 2;
    }
    config_file = argv[optind];

    /* Parse configuration */
    if (config_parse_file(config_file, &config) != 0) {
        fprintf(stderr, "Failed to parse configuration file\n");
        return 2;
    }

    /* Apply command line overrides */
    optind = 1; /* Reset for second pass */
    while ((opt = getopt_long(argc, argv, "vqfc:h", long_options, NULL)) != -1) {
        switch (opt) {
            case 'v':
                config.verbosity = 2;
                break;
            case 'q':
                config.verbosity = 0;
                break;
            case 'f':
                config.stop_on_fail = true;
                break;
        }
    }

    /* Initialize framework */
    framework_init();

    /* Register tests based on category */
    if (strcmp(category, "all") == 0 || strcmp(category, "discovery") == 0) {
        register_discovery_tests();
    }
    if (strcmp(category, "all") == 0 || strcmp(category, "commands") == 0) {
        register_command_tests();
    }
    if (strcmp(category, "all") == 0 || strcmp(category, "io") == 0) {
        register_io_tests();
    }

    /* Run tests */
    ret = framework_run_tests(&config);

    /* Cleanup */
    framework_cleanup();
    config_free(&config);

    return ret;
}
