#include <windows.h>

__declspec(dllimport) void greet(void);

void __main(void) {}

int main(void) {
    greet();
    ExitProcess(0);
    return 0;
}
