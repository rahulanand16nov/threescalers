#include "../capi/include/threescalers/threescalers.h"
#include <stdio.h>

int main() {
  char s[64];

  int r = encoding_encode("ho?tiajederya", &s[0], 64);
  if (r < 0) {
    fprintf(stderr, "failed encoding_encode");
  }
  printf("1st run: %s\n", s);
  const FFICow *fc = encoding_encode_s("ho?tiajoderya");
  if (fc->tag == Borrowed) {
    printf("borrowed!\n");
    printf("str: %s\n", fc->borrowed);
  } else {
    printf("owned\n");
    printf("own: %12s\n", fc->owned.ptr);
  }

  fficow_free(fc);
  printf("freed\n");

  return 0;
}