// main.c — dllimport あり/なしの 2 通りで DLL 関数を呼び出す
//
// 呼び出し方の違い:
//   __declspec(dllimport) あり → コンパイラが __imp__add_from_dll シンボルを参照
//                                リンカは IAT を直接参照するコードを生成する
//
//   __declspec(dllimport) なし → コンパイラが _mul_from_dll シンボルへの CALL を生成
//                                リンカは .dlljmp セクションに "FF 25 <IAT>" スタブを置き、
//                                CALL をそのスタブに向ける
//
// ImportResult ダンプで imp_symbol_to_iat_rva と symbol_to_jmp_va の両方が埋まることを確認できる。

#include <windows.h>

void __main(void) {}

// IAT 直接参照: __imp__add_from_dll シンボルを使う
__declspec(dllimport) int add_from_dll(int a, int b);

// JMP スタブ経由: _mul_from_dll → .dlljmp スタブ → IAT
int mul_from_dll(int a, int b);

int main(void) {
    int sum  = add_from_dll(6, 7); // dllimport: IAT 直接
    int prod = mul_from_dll(6, 7); // no dllimport: .dlljmp スタブ経由

    char msg[128];
    wsprintfA(msg, "add_from_dll(6,7) = %d  [dllimport / IAT direct]\n"
                   "mul_from_dll(6,7) = %d  [no dllimport / jmp stub]",
              sum, prod);
    MessageBoxA(NULL, msg, "jmp stub demo", MB_OK);
    ExitProcess(0);
    return 0;
}
