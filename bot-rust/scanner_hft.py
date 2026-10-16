import requests, time, os

print("[ORCHESTRATEUR] Phase 8 - Disjoncteur Thermique et Arrêt Automatique AWS...")
WETH = "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"

TOKENS = {
 "RAIN": "0x25118290e6A5f4139381D072181157035864099d",
 "NOUVEAU_CONTRAT": "0x6985884C4392D348587B19cb9eAAf157F13271cd",
 "ARB": "0x912CE59144191C1204E64559FE8253A0e49E6548"
}

IDLE_CYCLES = 0
MAX_IDLE_CYCLES = 720 # Seuil de rupture financière : ~60 minutes de scan stérile

def inject(t_addr, p_a, p_b):
 global IDLE_CYCLES
 IDLE_CYCLES = 0 # Remise à zéro du disjoncteur OS
 print(f"🔥 OPPORTUNITÉ DÉTECTÉE | Spread massif sur: {t_addr}")
 os.system("sed -i '/^TARGET_POOL_A=/d; /^TARGET_POOL_B=/d; /^TOKEN_A=/d; /^TOKEN_B=/d' .env")
 os.system(f"echo 'TOKEN_A=\"{WETH}\"\nTOKEN_B=\"{t_addr}\"\nTARGET_POOL_A=\"{p_a}\"\nTARGET_POOL_B=\"{p_b}\"' >> .env")
 os.system("pm2 restart arb-hft-bot --update-env --silent")

while True:
 if IDLE_CYCLES >= MAX_IDLE_CYCLES:
  print("[ALERTE] Rentabilité non atteinte. Coupure de l'instance AWS pour préservation du capital.")
  os.system("sudo shutdown -h now")
  time.sleep(60)

 has_opportunity = False
 for name, addr in TOKENS.items():
  try:
   r = requests.get(f"https://api.dexscreener.com/latest/dex/tokens/{addr}", timeout=5).json()
   pools = [p for p in r.get('pairs', []) if p.get('chainId') == 'arbitrum' and p.get('liquidity', {}).get('usd', 0) > 3000]
   
   if len(pools) >= 2:
    p1, p2 = pools[0], pools[1]
    px1, px2 = float(p1['priceUsd']), float(p2['priceUsd'])
    
    if px1 > 0 and px2 > 0:
     spread = abs(px1 - px2) / min(px1, px2) * 100
     print(f"[{name}] Spread Brut: {spread:.3f}% | Cycles stériles: {IDLE_CYCLES}/{MAX_IDLE_CYCLES}")
     
     if 1.5 < spread <= 15.0:
      inject(addr, p1['pairAddress'], p2['pairAddress'])
      has_opportunity = True
      print("[SCANNER] Exécution on-chain envoyée. Veille thermique (120s)...")
      time.sleep(120)
  except: pass
 
 if not has_opportunity:
  IDLE_CYCLES += 1
  
 time.sleep(5)
