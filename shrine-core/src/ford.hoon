
|%
++  swel
  |=  [tap=(trap vase) gen=hoon]
  ^-  (trap vase)
  =/  gun  (~(mint ut p:$:tap) %noun gen)
  =>  [tap=tap gun=gun]
  |.  ~+
  =/  pro  q:$:tap
  [[%cell p.gun p:$:tap] [.*(pro q.gun) pro]]
++  slup
  |=  [sin=(trap vase) dex=(trap vase)]
  ^-  (trap vase)
  =>  [sin=sin dex=dex]
  |.  ~+
  =/  sun  $:sin
  =/  dux  $:dex
  [[%cell -.sun -.dux] [+.sun +.dux]]
::

+$  face  term
+$  pile
  $:  pro=(list [=face =stud:t])
      vaz=(list [=face =pith:t])
  ==
+$  seam  [writ=? opt=?]
+$  wool
  [type=(set stud:t) =seam]
+$  yarn
  (map slot:t wool)
+$  zeta  (each iota [fac=(unit term) =aura])
+$  rail  (list zeta)
+$  fear
  $%  [%x =yarn]
      [%y =yarn kids=(map rail yarn)]
      [%z =yarn]
  ==
+$  spec
  $:  =yarn
      kids=(map rail yarn)
      fief=(map slot:t fear)
  ==
::
+$  loon
  [=pile =hoon]
+$  tome
  [=pile [=spec =hoon]]
++  parsers
  |_  wer=path
  ++  pile
    %+  ifix  [gay gay]
    ;~  plug
      (star pro)
      (star vax)
    ==
  ++  lon  ;~(plug pile hon)
  ++  hon
    =+  vaz=vast
    (ifix [gay gay] tall:vaz(wer wer))
  ++  ret  (just '\0a')
  ++  pit  ;~(pfix fas (most fas spot:stip))
  ++  std  ;~(pfix fas ;~(plug spot:stip pit))
  ++  eol  ;~(plug (star ace) ret)
  ++  seps   |=(rune=cord [;~(plug (just rune) gap) eol])
  ++  pro
    %+  ifix  (seps '/@')
    ;~  (glue gap)
      sym
      ;~(pose sym std)
    ==
  ++  vax
    %+  ifix  (seps '/-')
    ;~  (glue gap)
      sym
      pit
    ==
  --
::
::  Like +rose except also produces line number
::
++  lily
  |*  [los=tape sab=rule]
  =+  vex=(sab [[1 1] los])
  ?~  q.vex
    [%| p=p.vex(q (dec q.p.vex))]
  ?.  =(~ q.q.u.q.vex)
    [%| p=p.vex(q (dec q.p.vex))]
  [%& p=p.u.q.vex]
++  parse-loon
  |=  [=pith:t txt=@]
  ^-  (each loon hair)
  (lily (trip txt) ~(lon parsers (pout pith)))
--
