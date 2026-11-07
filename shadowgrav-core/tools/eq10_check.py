import numpy as np
a=0.01; d=1.0; tau=1e-6; beta=tau/(2*a)
chat=np.array([0.6,0,0.8]); c=chat*d
def basis(u):
    ax=np.array([1.,0,0]) if abs(u[0])<0.9 else np.array([0,1.,0])
    e1=np.cross(u,ax); e1/=np.linalg.norm(e1); return e1,np.cross(u,e1)
def chord(q):                      # q: (...,3) unit; ray from origin, sphere at c radius a
    t=q@c; b2=c@c-t*t; h2=a*a-b2
    L=np.where((h2>0)&(t>0),2*np.sqrt(np.maximum(h2,0)),0.0); return L
def V(ek,ej,N=6000,M=96):
    # n-space polar grid about n0; integrate g(n;ek) D(q(n)) dOmega_n  (no Jacobian formula used)
    lam=chat@ej+np.sqrt((chat@ej)**2+1-ej@ej); n0=lam*chat-ej; n0/=np.linalg.norm(n0)
    e1,e2=basis(n0); R=1.6*a/d
    th=(np.arange(N)+0.5)/N*R; ph=2*np.pi*(np.arange(M)+0.5)/M
    TH,PH=np.meshgrid(th,ph,indexing='ij')
    n=np.cos(TH)[...,None]*n0+np.sin(TH)[...,None]*(np.cos(PH)[...,None]*e1+np.sin(PH)[...,None]*e2)
    s=n+ej; q=s/np.linalg.norm(s,axis=-1,keepdims=True)
    D=-np.expm1(-beta*chord(q))
    g=n+ek; g=g*np.linalg.norm(g,axis=-1,keepdims=True)
    w=np.sin(TH)*(R/N)*(2*np.pi/M)
    return (g*(D*w)[...,None]).sum((0,1))
Aeff=np.pi*a*a*(1-2/tau**2*(1-(1+tau)*np.exp(-tau)))
# use series-accurate A: thin limit kappa*m = beta*(4/3 pi a^3)
Aeff=beta*4/3*np.pi*a**3
for sc in (1e-1,5e-2,2.5e-2):
    ek=sc*np.array([0.8,0.4,-0.6]); ej=sc*np.array([-0.5,0.7,0.3])
    got=V(ek,ej)/(Aeff/d**2)
    want=chat*(1+chat@(3*ej+ek))+ek-ej
    print(sc, np.linalg.norm(got-want)/np.linalg.norm(want), (np.linalg.norm(got-want)/np.linalg.norm(want))/max(np.linalg.norm(ek),np.linalg.norm(ej))**2)
