CC = gcc
CFLAGS = -Wall -Wextra -O2 -g -Isrc
LDFLAGS = -liscsi -lpthread

SRCDIR = src
OBJDIR = obj
BINDIR = .

# Source files
SRCS = $(wildcard $(SRCDIR)/*.c)
OBJS = $(SRCS:$(SRCDIR)/%.c=$(OBJDIR)/%.o)

TARGET = $(BINDIR)/iscsi-test-suite

.PHONY: all clean test install

all: $(TARGET)

$(TARGET): $(OBJS) | $(BINDIR)
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

$(OBJDIR)/%.o: $(SRCDIR)/%.c | $(OBJDIR)
	$(CC) $(CFLAGS) -c -o $@ $<

$(OBJDIR):
	mkdir -p $(OBJDIR)

$(BINDIR):
	mkdir -p $(BINDIR)

clean:
	rm -rf $(OBJDIR) $(TARGET)
	rm -f reports/*.txt

test: $(TARGET)
	./$(TARGET) config/test_config.ini

install: $(TARGET)
	install -m 755 $(TARGET) /usr/local/bin/

.PHONY: help
help:
	@echo "iSCSI Test Suite Build System"
	@echo "=============================="
	@echo "Targets:"
	@echo "  all     - Build the test suite (default)"
	@echo "  clean   - Remove build artifacts and reports"
	@echo "  test    - Build and run the test suite"
	@echo "  install - Install to /usr/local/bin"
	@echo "  help    - Show this help message"
