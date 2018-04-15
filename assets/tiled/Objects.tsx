<?xml version="1.0" encoding="UTF-8"?>
<tileset name="Objects" tilewidth="231" tileheight="223" tilecount="7" columns="0">
 <grid orientation="orthogonal" width="1" height="1"/>
 <properties>
  <property name="palette" type="file" value="../images/char_palette.png"/>
 </properties>
 <tile id="1">
  <properties>
   <property name="script" type="file" value="../scripts/player.lua"/>
  </properties>
  <image width="231" height="223" source="../images/walktest12-0.png"/>
  <animation>
   <frame tileid="1" duration="80"/>
   <frame tileid="2" duration="80"/>
   <frame tileid="3" duration="80"/>
   <frame tileid="5" duration="80"/>
   <frame tileid="6" duration="80"/>
   <frame tileid="7" duration="80"/>
  </animation>
 </tile>
 <tile id="2">
  <image width="231" height="223" source="../images/walktest12-1.png"/>
  <objectgroup draworder="index">
   <object id="7" type="Hitbox" x="43" y="12" width="126.5" height="199"/>
   <object id="8" type="Hurtbox" x="36.5" y="130" width="179.5" height="67.5"/>
  </objectgroup>
 </tile>
 <tile id="3">
  <image width="231" height="223" source="../images/walktest12-2.png"/>
 </tile>
 <tile id="4">
  <image width="231" height="223" source="../images/walktest12-3.png"/>
 </tile>
 <tile id="5">
  <image width="231" height="223" source="../images/walktest12-4.png"/>
 </tile>
 <tile id="6">
  <image width="231" height="223" source="../images/walktest12-5.png"/>
 </tile>
 <tile id="7">
  <image width="231" height="223" source="../images/walktest12-6.png"/>
 </tile>
</tileset>
