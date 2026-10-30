#!/usr/bin/env bash
set -e

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_SDK="${1:-${LOCI_SDK_DIR:-$ROOT_DIR/../sdk-love2d}}"
SOURCE_PROTO="$ROOT_DIR/proto/game_packets.proto"

echo "=== Loci2D - Synchronize Protobuf Schema to SDK ==="

if [ ! -f "$SOURCE_PROTO" ]; then
    echo "Erro: Arquivo proto fonte nao encontrado em $SOURCE_PROTO"
    exit 1
fi

if [ ! -d "$TARGET_SDK" ]; then
    echo "Erro: Repositorio alvo do SDK nao encontrado em: $TARGET_SDK"
    echo "Uso: ./tools/sync_sdk_proto.sh [caminho_para_sdk_love2d]"
    exit 1
fi

TARGET_LIB="$TARGET_SDK/lib"
mkdir -p "$TARGET_LIB"

echo "-> Copiando schema canonical para $TARGET_LIB/game_packets.proto..."
cp "$SOURCE_PROTO" "$TARGET_LIB/game_packets.proto"

# Compilar descritor binario .pb se protoc estiver instalado
TARGET_PB="$TARGET_LIB/game_packets.pb"
if command -v protoc >/dev/null 2>&1; then
    echo "-> Compilando descritor binario (game_packets.pb) via protoc..."
    protoc --proto_path="$ROOT_DIR/proto" \
           --descriptor_set_out="$TARGET_PB" \
           --include_imports \
           "$SOURCE_PROTO"
    echo "[OK] Descritor binario game_packets.pb atualizado com sucesso!"
else
    echo "Aviso: 'protoc' nao encontrado no PATH."
    echo "O arquivo .proto foi copiado, mas o .pb nao foi recompilado."
    echo "O SDK ainda funcionara perfeitamente atraves do parser dinamico (protoc.lua)."
fi

echo ""
echo "[OK] Sincronizacao concluida com sucesso!"
echo "Para commitar as alteracoes no SDK:"
echo "   cd $TARGET_SDK"
echo "   git status"
echo "   git commit -am 'chore(proto): sync game_packets.proto from engine core'"
echo ""
