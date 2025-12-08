#ifndef UTILS_H
#define UTILS_H

#include "test_framework.h"
#include <iscsi/iscsi.h>
#include <stdint.h>

/* Configuration file parsing */
int config_parse_file(const char *filename, test_config_t *config);
void config_free(test_config_t *config);

/* iSCSI connection helpers */
struct iscsi_context* create_iscsi_context_for_test(test_config_t *config);
int iscsi_connect_target(struct iscsi_context *iscsi, test_config_t *config);
void iscsi_disconnect_target(struct iscsi_context *iscsi);

/* Data pattern generation */
void generate_pattern(uint8_t *buffer, size_t size, const char *pattern_type, uint32_t seed);
int verify_pattern(const uint8_t *buffer, size_t size, const char *pattern_type, uint32_t seed);

/* SCSI helpers */
int scsi_read_capacity(struct iscsi_context *iscsi, int lun, uint64_t *num_blocks, uint32_t *block_size);
int scsi_read_blocks(struct iscsi_context *iscsi, int lun, uint64_t lba, uint32_t num_blocks,
                     uint32_t block_size, uint8_t *buffer);
int scsi_write_blocks(struct iscsi_context *iscsi, int lun, uint64_t lba, uint32_t num_blocks,
                      uint32_t block_size, const uint8_t *buffer);

/* String helpers */
char* trim_whitespace(char *str);
char* str_dup_safe(const char *str);

#endif /* UTILS_H */
