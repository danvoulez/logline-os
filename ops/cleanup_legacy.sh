#!/bin/bash

# LogLine Universe - Legacy Code Cleanup Script
# Remove código antigo/redundante, mantendo apenas a arquitetura nova

set -e

echo "🧹 Limpando código legado do LogLine Universe..."

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Função para deletar com confirmação
delete_item() {
    local item="$1"
    if [ -e "$item" ]; then
        echo -e "${YELLOW}🗑️  Deletando: $item${NC}"
        rm -rf "$item"
    else
        echo -e "${BLUE}⏭️  Já removido: $item${NC}"
    fi
}

echo -e "${BLUE}📋 Removendo pastas legadas/redundantes...${NC}"

# Código antigo/duplicado
delete_item "LogLine Network OS/"
delete_item "motor/"
delete_item "timeline/"
delete_item "rules/"
delete_item "modules/"
delete_item "time/"
delete_item "observer/"
delete_item "onboarding/"
delete_item "infra/"
delete_item "enforcement/"
delete_item "grammar/"

# Pastas vazias/experimentais
delete_item "agents/"
delete_item "contracts/"
delete_item "examples/"
delete_item "SEED/"
delete_item "receipts/"
delete_item "runtime/"
delete_item "schema/"
delete_item "test/"
delete_item "test_data/"
delete_item "ui/"
delete_item "target/"

# Arquivos de teste/protótipo soltos
delete_item "teste_enforcement.lll"
delete_item "recibo_loja_0001.lll"
delete_item "system.manifest.lll"
delete_item "manifesto_logline.lll"
delete_item "lib.rs"
delete_item "setup_postgres.sh"

# Arquivos temporários
delete_item ".DS_Store"

# Scripts antigos (manter apenas os úteis)
delete_item "scripts/teste_enforcement.sh"
delete_item "scripts/unificar_projeto.sh"

echo -e "${BLUE}📋 Limpando arquivos temporários do Railway...${NC}"
delete_item "RAILWAY_SETUP_GUIDE.md"  # Duplicado, manter só RAILWAY_MANUAL_SETUP.md
delete_item "railway.json"  # Vazio, será recriado se necessário

echo -e "${GREEN}✅ Limpeza concluída!${NC}"

echo -e "${BLUE}📊 Estrutura final (código limpo):${NC}"
echo "✅ Serviços principais:"
echo "   • logline-core/"
echo "   • logline-engine/"
echo "   • logline-rules/"
echo "   • logline-timeline/"
echo "   • logline-id/"
echo "   • logline-protocol/"

echo "✅ Infraestrutura:"
echo "   • cli/"
echo "   • federation/"
echo "   • migrations/"
echo "   • tests/"
echo "   • fuzz/"
echo "   • docs/"

echo "✅ Deploy:"
echo "   • Dockerfile.*"
echo "   • scripts/railway_*.sh"
echo "   • scripts/run_migrations.sh"
echo "   • Makefile"

echo "✅ Documentação:"
echo "   • ROADMAP.md"
echo "   • TASKLIST.md"
echo "   • RAILWAY_MANUAL_SETUP.md"

echo -e "${GREEN}🎉 Repositório limpo e organizado!${NC}"
echo -e "${YELLOW}📝 Próximos passos:${NC}"
echo "1. git add -A"
echo "2. git commit -m 'Clean up legacy code, keep only new WebSocket mesh architecture'"
echo "3. git push origin main"

# Mostrar estatísticas
echo -e "${BLUE}📈 Estatísticas do repositório limpo:${NC}"
echo "• Serviços: 6 (core + 5 microservices)"
echo "• Testes: $(find tests/ -name "*.rs" 2>/dev/null | wc -l | tr -d ' ') arquivos"
echo "• Docs: $(find docs/ -name "*.md" 2>/dev/null | wc -l | tr -d ' ') arquivos"
echo "• Dockerfiles: $(ls Dockerfile.* 2>/dev/null | wc -l | tr -d ' ') serviços"
