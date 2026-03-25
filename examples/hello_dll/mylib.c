#include <windows.h>

__declspec(dllexport) void greet(void) {
    MessageBoxA(NULL, "Hello from mylib.dll!", "Custom DLL Example", MB_OK);
}
