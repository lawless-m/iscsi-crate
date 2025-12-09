#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <iscsi/iscsi.h>
#include <iscsi/scsi-lowlevel.h>

/* Trim whitespace from string */
char* trim_whitespace(char *str) {
    char *end;

    /* Trim leading space */
    while (isspace((unsigned char)*str)) str++;

    if (*str == 0) return str;

    /* Trim trailing space */
    end = str + strlen(str) - 1;
    while (end > str && isspace((unsigned char)*end)) end--;

    end[1] = '\0';
    return str;
}

/* Safe string duplication */
char* str_dup_safe(const char *str) {
    if (!str) return NULL;
    return strdup(str);
}

/* Parse INI file */
int config_parse_file(const char *filename, test_config_t *config) {
    FILE *f = fopen(filename, "r");
    if (!f) {
        fprintf(stderr, "Failed to open config file: %s\n", filename);
        return -1;
    }

    char line[1024];
    char section[128] = "";

    /* Set defaults */
    memset(config, 0, sizeof(test_config_t));
    config->lun = 0;
    config->block_size = 512;
    config->large_transfer_blocks = 1024;
    config->timeout = 30;
    config->stress_iterations = 100;
    config->verbosity = 1;
    config->stop_on_fail = false;
    config->generate_report = true;

    while (fgets(line, sizeof(line), f)) {
        char *trimmed = trim_whitespace(line);

        /* Skip empty lines and comments */
        if (trimmed[0] == '\0' || trimmed[0] == '#' || trimmed[0] == ';') {
            continue;
        }

        /* Section header */
        if (trimmed[0] == '[') {
            char *end = strchr(trimmed, ']');
            if (end) {
                *end = '\0';
                strncpy(section, trimmed + 1, sizeof(section) - 1);
                section[sizeof(section) - 1] = '\0';
            }
            continue;
        }

        /* Key=value pair */
        char *equals = strchr(trimmed, '=');
        if (!equals) continue;

        *equals = '\0';
        char *key = trim_whitespace(trimmed);
        char *value = trim_whitespace(equals + 1);

        /* Parse based on section */
        if (strcmp(section, "target") == 0) {
            if (strcmp(key, "portal") == 0) {
                config->portal = strdup(value);
            } else if (strcmp(key, "iqn") == 0) {
                config->iqn = strdup(value);
            } else if (strcmp(key, "lun") == 0) {
                config->lun = atoi(value);
            }
        } else if (strcmp(section, "authentication") == 0) {
            if (strcmp(key, "auth_method") == 0) {
                config->auth_method = strdup(value);
            } else if (strcmp(key, "username") == 0) {
                config->username = strdup(value);
            } else if (strcmp(key, "password") == 0) {
                config->password = strdup(value);
            } else if (strcmp(key, "mutual_username") == 0) {
                config->mutual_username = strdup(value);
            } else if (strcmp(key, "mutual_password") == 0) {
                config->mutual_password = strdup(value);
            }
        } else if (strcmp(section, "test_parameters") == 0) {
            if (strcmp(key, "block_size") == 0) {
                config->block_size = atoi(value);
            } else if (strcmp(key, "large_transfer_blocks") == 0) {
                config->large_transfer_blocks = atoi(value);
            } else if (strcmp(key, "timeout") == 0) {
                config->timeout = atoi(value);
            } else if (strcmp(key, "stress_iterations") == 0) {
                config->stress_iterations = atoi(value);
            }
        } else if (strcmp(section, "options") == 0) {
            if (strcmp(key, "verbosity") == 0) {
                config->verbosity = atoi(value);
            } else if (strcmp(key, "stop_on_fail") == 0) {
                config->stop_on_fail = (strcmp(value, "true") == 0 || strcmp(value, "1") == 0);
            } else if (strcmp(key, "generate_report") == 0) {
                config->generate_report = (strcmp(value, "true") == 0 || strcmp(value, "1") == 0);
            }
        }
    }

    fclose(f);

    /* Validate required fields */
    if (!config->portal) {
        fprintf(stderr, "Error: portal not specified in config file\n");
        return -1;
    }

    return 0;
}

/* Free configuration */
void config_free(test_config_t *config) {
    if (config->portal) free(config->portal);
    if (config->iqn) free(config->iqn);
    if (config->auth_method) free(config->auth_method);
    if (config->username) free(config->username);
    if (config->password) free(config->password);
    if (config->mutual_username) free(config->mutual_username);
    if (config->mutual_password) free(config->mutual_password);
    memset(config, 0, sizeof(test_config_t));
}

/* Create iSCSI context */
struct iscsi_context* create_iscsi_context_for_test(test_config_t *config) {
    struct iscsi_context *iscsi;
    struct iscsi_url *iscsi_url;
    char url[512];

    /* Build iSCSI URL */
    if (config->iqn && strlen(config->iqn) > 0) {
        snprintf(url, sizeof(url), "iscsi://%s/%s/%d",
                 config->portal, config->iqn, config->lun);
    } else {
        snprintf(url, sizeof(url), "iscsi://%s",
                 config->portal);
    }

    iscsi_url = iscsi_parse_full_url(NULL, url);
    if (!iscsi_url) {
        return NULL;
    }

    iscsi = iscsi_create_context("iqn.2024-12.com.test:initiator");
    if (!iscsi) {
        iscsi_destroy_url(iscsi_url);
        return NULL;
    }

    iscsi_set_targetname(iscsi, iscsi_url->target);
    iscsi_set_session_type(iscsi, ISCSI_SESSION_NORMAL);
    iscsi_set_header_digest(iscsi, ISCSI_HEADER_DIGEST_NONE);

    /* Set authentication if configured */
    if (config->auth_method) {
        if (strcmp(config->auth_method, "chap") == 0 || strcmp(config->auth_method, "mutual_chap") == 0) {
            if (config->username && config->password) {
                iscsi_set_initiator_username_pwd(iscsi, config->username, config->password);
            }
        }
        if (strcmp(config->auth_method, "mutual_chap") == 0) {
            if (config->mutual_username && config->mutual_password) {
                iscsi_set_target_username_pwd(iscsi, config->mutual_username, config->mutual_password);
            }
        }
    }

    iscsi_destroy_url(iscsi_url);
    return iscsi;
}

/* Connect to target */
int iscsi_connect_target(struct iscsi_context *iscsi, test_config_t *config) {
    struct iscsi_url *iscsi_url;
    char url[512];
    int ret;

    if (config->iqn && strlen(config->iqn) > 0) {
        snprintf(url, sizeof(url), "iscsi://%s/%s/%d",
                 config->portal, config->iqn, config->lun);
    } else {
        snprintf(url, sizeof(url), "iscsi://%s",
                 config->portal);
    }

    iscsi_url = iscsi_parse_full_url(iscsi, url);
    if (!iscsi_url) {
        return -1;
    }

    ret = iscsi_full_connect_sync(iscsi, iscsi_url->portal, iscsi_url->lun);
    iscsi_destroy_url(iscsi_url);

    return ret;
}

/* Disconnect from target */
void iscsi_disconnect_target(struct iscsi_context *iscsi) {
    if (iscsi) {
        iscsi_logout_sync(iscsi);
        iscsi_disconnect(iscsi);
    }
}

/* Generate data pattern */
void generate_pattern(uint8_t *buffer, size_t size, const char *pattern_type, uint32_t seed) {
    if (strcmp(pattern_type, "zero") == 0) {
        memset(buffer, 0x00, size);
    } else if (strcmp(pattern_type, "ones") == 0) {
        memset(buffer, 0xFF, size);
    } else if (strcmp(pattern_type, "alternating") == 0) {
        for (size_t i = 0; i < size; i++) {
            buffer[i] = (i % 2) ? 0xAA : 0x55;
        }
    } else if (strcmp(pattern_type, "sequential") == 0) {
        for (size_t i = 0; i < size; i++) {
            buffer[i] = (uint8_t)(i & 0xFF);
        }
    } else if (strcmp(pattern_type, "random") == 0) {
        srand(seed);
        for (size_t i = 0; i < size; i++) {
            buffer[i] = (uint8_t)(rand() & 0xFF);
        }
    } else {
        /* Default: sequential */
        for (size_t i = 0; i < size; i++) {
            buffer[i] = (uint8_t)(i & 0xFF);
        }
    }
}

/* Verify data pattern */
int verify_pattern(const uint8_t *buffer, size_t size, const char *pattern_type, uint32_t seed) {
    uint8_t *expected = malloc(size);
    if (!expected) {
        return -1;
    }

    generate_pattern(expected, size, pattern_type, seed);
    int result = memcmp(buffer, expected, size);
    free(expected);

    return (result == 0) ? 0 : -1;
}

/* Read capacity */
int scsi_read_capacity(struct iscsi_context *iscsi, int lun, uint64_t *num_blocks, uint32_t *block_size) {
    struct scsi_task *task;

    task = iscsi_readcapacity10_sync(iscsi, lun, 0, 0);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        if (task) {
            scsi_free_scsi_task(task);
        }
        return -1;
    }

    /* Parse READ CAPACITY(10) response from datain buffer */
    if (task->datain.size < 8) {
        scsi_free_scsi_task(task);
        return -1;
    }

    unsigned char *buf = task->datain.data;
    uint32_t last_lba = (buf[0] << 24) | (buf[1] << 16) | (buf[2] << 8) | buf[3];
    uint32_t blk_size = (buf[4] << 24) | (buf[5] << 16) | (buf[6] << 8) | buf[7];

    *num_blocks = (uint64_t)last_lba + 1;
    *block_size = blk_size;

    scsi_free_scsi_task(task);
    return 0;
}

/* Read blocks */
int scsi_read_blocks(struct iscsi_context *iscsi, int lun, uint64_t lba, uint32_t num_blocks,
                     uint32_t block_size, uint8_t *buffer) {
    struct scsi_task *task;

    task = iscsi_read10_sync(iscsi, lun, lba, num_blocks * block_size, block_size, 0, 0, 0, 0, 0);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        if (task) {
            scsi_free_scsi_task(task);
        }
        return -1;
    }

    memcpy(buffer, task->datain.data, num_blocks * block_size);
    scsi_free_scsi_task(task);
    return 0;
}

/* Write blocks */
int scsi_write_blocks(struct iscsi_context *iscsi, int lun, uint64_t lba, uint32_t num_blocks,
                      uint32_t block_size, const uint8_t *buffer) {
    struct scsi_task *task;

    task = iscsi_write10_sync(iscsi, lun, lba, (unsigned char *)buffer,
                              num_blocks * block_size, block_size, 0, 0, 0, 0, 0);
    if (!task || task->status != SCSI_STATUS_GOOD) {
        if (task) {
            scsi_free_scsi_task(task);
        }
        return -1;
    }

    scsi_free_scsi_task(task);
    return 0;
}
