{
  description = "Munix Core Flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};

  in {
    packages.${system} = {
      env = pkgs.symlinkJoin {
        name = "munix-env";
        paths = [
          pkgs.shntool
          pkgs.cuetools
          pkgs.flac
          pkgs.imagemagick
          pkgs.jq
          pkgs.stdenv
          nixpkgs
        ];
      };
    };

    lib = {
      evalConfig = albumArgs: let
        envOrigin = builtins.getEnv "MUNIX_ORIGIN_PATH";
        envSourceName = builtins.getEnv "MUNIX_SOURCE_NAME";
        
        origin = {
          path = if ((albumArgs.origin.path or "") != "") then albumArgs.origin.path else envOrigin;
          hash = albumArgs.origin.hash or "";
        };
        
        source = {
          torrent = {
            file = albumArgs.source.torrent.file or "";
            name = if ((albumArgs.source.torrent.name or "") != "") then albumArgs.source.torrent.name else envSourceName;
            hash = albumArgs.source.torrent.hash or "";
          };
          web = {
            url = albumArgs.source.web.url or "";
            hash = albumArgs.source.web.hash or "";
          };
        };
      in builtins.scopedImport { inherit origin source; } ./config.nix;

      splitCueImage = { name ? "split", cue, image }: pkgs.stdenv.mkDerivation {
        inherit name cue image;
        buildInputs = [ pkgs.shntool pkgs.cuetools pkgs.flac pkgs.imagemagick pkgs.jq ];
        unpackPhase = "true";
        buildPhase = ''
          mkdir -p $out
          shnsplit -f "$cue" -o flac -t "%n" -d $out "$image"
        '';
        installPhase = "true";
      };

      mkCover = { name, src, relPath ? null }: pkgs.stdenv.mkDerivation {
        inherit name src relPath;
        buildInputs = [ pkgs.imagemagick ];
        unpackPhase = "true";
        buildPhase = ''
          INPUT_FILE="${if relPath == null then "$src" else "$src/$relPath"}"
          magick "$INPUT_FILE" -filter Mitchell -thumbnail 1080x1080^ -gravity center -extent 1080x1080 cover.png
        '';
        installPhase = ''
          mkdir -p $out
          cp cover.png $out/cover.png
        '';
      };

      mkTrack = { name, src, relPath, metadata ? {}, cover ? null, allowedTags ? [] }: let
        filteredMeta = if allowedTags == [] then metadata else pkgs.lib.filterAttrs (k: v: builtins.elem (pkgs.lib.toLower k) allowedTags) metadata;
        metaJson = pkgs.writeText "meta.json" (builtins.toJSON filteredMeta);
      in pkgs.stdenv.mkDerivation {
        inherit name src relPath;
        buildInputs = [ pkgs.flac pkgs.jq ];
        unpackPhase = "true";
        buildPhase = ''
          cp "$src/$relPath" track.flac
          chmod +w track.flac
          metaflac --remove-all-tags track.flac
          
          jq -r 'to_entries | .[] | if (.value | type) == "array" then .key as $k | .value[] | "\($k)=\(.)" else "\(.key)=\(.value)" end' ${metaJson} > tags.txt
          while IFS= read -r tag; do
            metaflac --set-tag="$tag" track.flac
          done < tags.txt

          ${if cover != null then ''metaflac --import-picture-from="${cover}/cover.png" track.flac'' else ""}
        '';
        installPhase = ''
          mkdir -p $out
          cp track.flac $out/track.flac
        '';
      };

      mkAlbum = args@{ 
        name, 
        origin ? { path = ""; hash = ""; },
        source ? {},
        album ? { metadata = {}; mbid = {}; },
        tracks ? [], 
        cover ? null
      }: let
        getMergedTrackMeta = t: (album.metadata or {}) // (album.mbid or {}) // (t.metadata or {}) // (t.mbid or {});

        trackIds = builtins.map (t: let m = getMergedTrackMeta t; in "${toString (m.discnumber or 1)}-${toString (m.tracknumber or 0)}") tracks;
        uniqueTrackIds = pkgs.lib.unique trackIds;
        hasDuplicates = builtins.length trackIds != builtins.length uniqueTrackIds;

        maxDisc = builtins.foldl' (acc: t: pkgs.lib.max acc ((getMergedTrackMeta t).discnumber or 1)) 1 tracks;
        maxTrack = builtins.foldl' (acc: t: pkgs.lib.max acc ((getMergedTrackMeta t).tracknumber or 0)) 1 tracks;
        
        discPadLen = builtins.stringLength (toString maxDisc);
        trackPadLen = pkgs.lib.max 2 (builtins.stringLength (toString maxTrack));

        config = self.lib.evalConfig args;

        toTomlVal = v:
          if builtins.isString v then "\"${pkgs.lib.escape ["\"" "\\"] v}\""
          else if builtins.isInt v then toString v
          else if builtins.isBool v then (if v then "true" else "false")
          else if builtins.isList v then "[ " + pkgs.lib.concatMapStringsSep ", " toTomlVal v + " ]"
          else "\"\"";
        
        toTomlTable = order: data: let
          orderedLines = pkgs.lib.concatMap (pathStr:
            let
              parts = pkgs.lib.splitString "." pathStr;
              manifest = builtins.elemAt parts 0;
              key = builtins.elemAt parts 1;
            in if builtins.isAttrs (data.${manifest} or null) && data.${manifest} ? ${key}
               then [ "${key} = ${toTomlVal data.${manifest}.${key}}" ]
               else []
          ) order;
        in pkgs.lib.concatStringsSep "\n" orderedLines;

        metadataToml = let
          aTable = toTomlTable (config.keys.album or []) album;
          aS = if aTable != "" then "[album]\n${aTable}" else "";
          tS = pkgs.lib.concatMapStringsSep "\n\n" (t: let table = toTomlTable (config.keys.tracks or []) t; in if table != "" then "[[tracks]]\n${table}" else "[[tracks]]") tracks;
          sep = if aS != "" && tS != "" then "\n\n" else "";
        in pkgs.writeText "metadata.toml" (aS + sep + tS + "\n");

        envOrigin = builtins.getEnv "MUNIX_ORIGIN_PATH";
        envSanitizedSourceName = builtins.getEnv "MUNIX_SANITIZED_SOURCE_NAME";
        srcBaseName = if envSanitizedSourceName != "" then envSanitizedSourceName else name;
        
        resolvedOriginPath = 
          if (origin.path or "") != "" then origin.path 
          else if envOrigin != "" then envOrigin
          else config.origin;

        rawSrcPath = if resolvedOriginPath != "" then resolvedOriginPath
                     else throw "Origin path missing in album.nix, config.nix, and environment";

        realSrc = 
          let
            isStore = builtins.isString rawSrcPath && builtins.match "/nix/store/[0-9abcdfghijklmnpqrsvwxyz]{32}-.*" rawSrcPath != null;
          in
          if isStore then 
            builtins.storePath (builtins.elemAt (builtins.match ".*(/nix/store/[0-9abcdfghijklmnpqrsvwxyz]{32}-.*)" rawSrcPath) 0)
          else
            let
              pathObj = if builtins.isPath rawSrcPath then rawSrcPath
                        else if builtins.isString rawSrcPath && pkgs.lib.hasPrefix "/" rawSrcPath then /. + rawSrcPath
                        else rawSrcPath;
            in
            if builtins.isPath pathObj then
              builtins.path { 
                name = "${srcBaseName}-source"; 
                path = pathObj; 
                sha256 = origin.hash; 
              }
            else 
              pathObj;

        processedCover = if cover != null && cover.file != null
                         then (
                           if builtins.isPath cover.file
                           then let
                             realCoverPath = if (cover.hash or "") != "" then
                               builtins.path { 
                                 name = "${name}-cover-src"; 
                                 path = cover.file; 
                                 sha256 = cover.hash; 
                                 recursive = false;
                               }
                             else cover.file;
                           in self.lib.mkCover { name = "${name}-cover"; src = realCoverPath; }
                           else self.lib.mkCover { name = "${name}-cover"; src = realSrc; relPath = cover.file; }
                         )
                         else null;
        
        builtTracks = pkgs.lib.lists.imap1 (idx: track: let
          mergedMeta = getMergedTrackMeta track;
          disc = mergedMeta.discnumber or 1;
          trk = mergedMeta.tracknumber or 0;
          title = mergedMeta.title or "Untitled";
          discStr = pkgs.lib.fixedWidthString discPadLen "0" (toString disc);
          trkStr = pkgs.lib.fixedWidthString trackPadLen "0" (toString trk);
          fileName = if maxDisc == 1 then "${trkStr} - ${title}.flac" else "${discStr}.${trkStr} - ${title}.flac";
        in {
          inherit fileName;
          drv = self.lib.mkTrack {
            name = "${name}-disc${toString disc}-track${toString trk}";
            src = realSrc;
            relPath = track.file;
            metadata = mergedMeta;
            cover = processedCover;
            allowedTags = config.allowedTags or [];
          };
        }) tracks;

      in if hasDuplicates then throw "Duplicate discnumber and tracknumber combinations found in tracks." else pkgs.stdenv.mkDerivation {
        inherit name;
        src = realSrc;
        passthru = {
          sourceStorePath = realSrc;
          rawArgs = args;
        };
        unpackPhase = "true";
        buildPhase = ''
          mkdir -p $out
          ${pkgs.lib.strings.concatMapStringsSep "\n" (t: ''
            ln -s "${t.drv}/track.flac" "$out/${t.fileName}"
          '') builtTracks}
          cp ${metadataToml} $out/metadata.toml
        '';
        installPhase = "true";
      };
    };
  };
}
