# ADR 0007: Estratégia de Simulação Determinística e Aritmética de Ponto Fixo

## Status

Aceito — **Adiado para a Fase 4**

## Contexto

Para que o `loci2d` suporte **replays determinísticos** e **registro de partidas baseado em Event Sourcing** (Fase 4), a simulação do jogo precisa ser executada de forma estritamente idêntica em qualquer plataforma (Windows, Linux, macOS) e arquitetura de CPU (x86_64, ARM64).

Durante a validação do MVP nas Fases 1–3, foram utilizadas operações de ponto flutuante padrão (`f32`) e tabelas hash padrão (`HashMap`) por simplicidade. No entanto, essas escolhas apresentam comportamentos não determinísticos conhecidos:

1. **Não Determinismo de Ponto Flutuante (`f32` / `f64`)**:
   - Regras de arredondamento de hardware, instruções FMA (Fused Multiply-Accumulate), tratamento de subnormais e extensões SIMD da CPU (x86 AVX/SSE vs ARM NEON) variam entre processadores.
   - Ao longo de centenas ou milhares de ticks, pequenas diferenças de arredondamento se acumulam e causam dessincronização de estado (efeito borboleta).

2. **Não Determinismo na Iteração de Coleções (`HashMap`)**:
   - O `HashMap` padrão do Rust utiliza `RandomState` com uma semente aleatória por processo (SipHash) para prevenir ataques HashDoS.
   - Iterar sobre entidades em um `HashMap` gera ordens de execução diferentes entre reinicializações do servidor e entre máquinas distintas.

3. **Granularidade do Sleep do Sistema Operacional**:
   - O tempo de `thread::sleep` varia conforme o kernel do SO (ex: resolução de interrupção de ~15.6 ms no Windows vs temporizadores de alta resolução de ~1 ms no Linux).

## Decisão

Decidimos manter `f32` e `HashMap` durante as fases iniciais do MVP (Fases 1–3) para priorizar a ergonomia de desenvolvimento, mas aplicar regras estritas de determinismo ao entrar na **Fase 4**:

1. **Aritmética de Ponto Fixo (`I32F32`) na Fase 4**:
   - Substituir coordenadas de vetores e física em `f32` por inteiros de ponto fixo (ex: crate `fixed` ou inteiros escalados de 32 bits).
   - Garantir resultados binários idênticos em qualquer arquitetura de CPU e compilador.

2. **Coleções de Entidades Ordenadas (`BTreeMap`)**:
   - Substituir `HashMap<u64, Entity>` por `BTreeMap<u64, Entity>` dentro do `Instance`.
   - Garantir uma ordem de iteração estrita e ordenada por chave (`1, 2, 3...`) em todas as plataformas.

3. **Game Loop com Acumulador de Timestep Fixo**:
   - Substituir o `thread::sleep(budget - elapsed)` ingênuo por um loop com acumulador de timestep fixo para eliminar a deriva (drift) entre sistemas operacionais.

4. **Log de Inputs Indexado por Tick**:
   - Registrar intenções juntamente com o número de tick em que foram executadas, garantindo que o replay consuma os inputs no quadro exato, independente do jitter de rede.

## Consequências

**Positivas:**
- **Replays 100% Determinísticos**: Logs de input geram hashes SHA256 de `WorldState` idênticos em x86_64, ARM64 e WebAssembly.
- **Paridade Cross-Platform**: O comportamento do servidor torna-se totalmente independente da resolução de temporizadores do SO hospedeiro.
- **Baixo Consumo de Memória**: O sistema de replay grava apenas as intenções dos clientes, sem a necessidade de salvar snapshots inteiros a cada tick.

**Negativas:**
- **Ergonomia do Ponto Fixo**: Converter operações matemáticas flutuantes para ponto fixo exige funções utilitárias personalizadas ou dependências de crates.
- **Custo de Busca no `BTreeMap`**: Custo de busca `O(log N)` comparado a `O(1)` no `HashMap` (desprezível para a quantidade de entidades por instância).
