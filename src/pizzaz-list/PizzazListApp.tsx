import { PlusCircle, Star } from "lucide-react";
import clsx from "clsx";
import markers from "../pizzaz/markers.json";
import { useDisplayMode } from "../use-display-mode";
import { useOpenAiGlobal } from "../use-openai-global";

type PizzazListToolOutput = {
  pizzaTopping?: string;
  city?: string;
};

type Props = {
  toolOutput?: PizzazListToolOutput | null;
};

export function PizzazListApp({ toolOutput: toolOutputProp }: Props) {
  const toolOutput =
    toolOutputProp ?? (useOpenAiGlobal("toolOutput") as PizzazListToolOutput | null);

  const allPlaces = markers?.places || [];
  const cityFilter = toolOutput?.city?.trim().toLowerCase();
  const places = cityFilter
    ? allPlaces.filter((p) => p.city?.toLowerCase().includes(cityFilter))
    : allPlaces;
  const displayMode = useDisplayMode();
  const theme = useOpenAiGlobal("theme");
  const isFullscreen = displayMode === "fullscreen";
  const isPip = displayMode === "pip";
  const isDark = theme === "dark";

  const rootClass = clsx(
    "antialiased w-full overflow-hidden transition-colors duration-200",
    isDark ? "bg-[#101828] text-white" : "bg-white text-black",
    isFullscreen
      ? "px-6 py-5 sm:rounded-none sm:border-none h-full"
      : clsx(
          "px-4 pb-2 rounded-2xl sm:rounded-3xl",
          isDark ? "border border-white/10" : "border border-black/10"
        )
  );

  const headlineLayoutClass = clsx(
    "flex flex-row items-center gap-4 sm:gap-4 border-b",
    isDark ? "border-white/10" : "border-black/5",
    isFullscreen ? "py-6" : "py-4"
  );

  const itemsToShow = isPip ? 3 : 7;
  const secondaryTextClass = isDark ? "text-white/60" : "text-black/60";
  const tertiaryTextClass = isDark ? "text-white/50" : "text-black/40";
  const hoverRowClass = isDark ? "hover:bg-white/10" : "hover:bg-black/5";
  const rowDividerColor = isDark ? "rgba(255, 255, 255, 0.12)" : "rgba(0, 0, 0, 0.05)";

  return (
    <div className={rootClass}>
      <div className="max-w-full">
        <div className={headlineLayoutClass}>
          <div
            className="sm:w-18 w-16 aspect-square rounded-xl bg-cover bg-center"
            style={{
              backgroundImage:
                "url(https://persistent.oaistatic.com/pizzaz/title.png)",
            }}
          ></div>
          <div>
            <div className="text-base sm:text-xl font-medium">
              National Best Pizza List
            </div>
            <div className={clsx("text-sm", secondaryTextClass)}>
              {toolOutput?.city
                ? `Top pizzerias near ${toolOutput.city}`
                : "A ranking of the best pizzerias in the world"}
            </div>
          </div>
          <div
            className={clsx(
              "flex-auto hidden sm:flex justify-end pr-2",
              isPip && "opacity-60"
            )}
          >
            <button
              type="button"
              className="cursor-pointer inline-flex items-center rounded-full bg-[#F46C21] text-white px-4 py-1.5 sm:text-md text-sm font-medium hover:opacity-90 active:opacity-100"
            >
              Save List
            </button>
          </div>
        </div>
        <div className="min-w-full text-sm flex flex-col">
          {places.slice(0, itemsToShow).map((place, i) => (
            <div
              key={place.id}
              className={clsx("px-3 -mx-2 rounded-2xl", hoverRowClass)}
            >
              <div
                style={{
                  borderBottom:
                    i === itemsToShow - 1 ? "none" : `1px solid ${rowDividerColor}`,
                }}
                className={clsx(
                  "flex w-full items-center gap-2",
                  isDark ? "hover:border-white/0!" : "hover:border-black/0!"
                )}
              >
                <div className="py-3 pr-3 min-w-0 w-full sm:w-3/5">
                  <div className="flex items-center gap-3">
                    <img
                      src={place.thumbnail}
                      alt={place.name}
                      className={clsx(
                        "h-10 w-10 sm:h-11 sm:w-11 rounded-lg object-cover ring",
                        isDark ? "ring-white/10" : "ring-black/5"
                      )}
                    />
                    <div
                      className={clsx(
                        "w-3 text-end sm:block text-sm",
                        tertiaryTextClass,
                        isPip ? "hidden" : "hidden sm:block"
                      )}
                    >
                      {i + 1}
                    </div>
                    <div className="min-w-0 sm:pl-1 flex flex-col items-start h-full">
                      <div className="font-medium text-sm sm:text-md truncate max-w-[40ch]">
                        {place.name}
                      </div>
                      <div
                        className={clsx(
                          "mt-1 sm:mt-0.25 flex items-center gap-3 text-sm",
                          isDark ? "text-white/70" : "text-black/70"
                        )}
                      >
                        <div className="flex items-center gap-1">
                          <Star
                            strokeWidth={1.5}
                            className={clsx("h-3 w-3", isDark ? "text-white" : "text-black")}
                          />
                          <span>
                            {place.rating?.toFixed
                              ? place.rating.toFixed(1)
                              : place.rating}
                          </span>
                        </div>
                        <div
                          className={clsx(
                            "whitespace-nowrap sm:hidden",
                            secondaryTextClass
                          )}
                        >
                          {place.city || "–"}
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
                <div
                  className={clsx(
                    "hidden sm:block text-end py-2 px-3 text-sm whitespace-nowrap flex-auto",
                    secondaryTextClass
                  )}
                >
                  {place.city || "–"}
                </div>
                <div className="py-2 whitespace-nowrap flex justify-end">
                  <PlusCircle
                    strokeWidth={1.5}
                    className={clsx("h-5 w-5", isPip && "opacity-70")}
                  />
                </div>
              </div>
            </div>
          ))}
          {places.length === 0 && (
            <div className={clsx("py-6 text-center", secondaryTextClass)}>
              No pizzerias found.
            </div>
          )}
        </div>
        <div className={clsx("px-0 pt-2 pb-2", isFullscreen ? "" : "sm:hidden")}>
          <button
            type="button"
            className="w-full cursor-pointer inline-flex items-center justify-center rounded-full bg-[#F46C21] text-white px-4 py-2 font-medium hover:opacity-90 active:opacity-100"
          >
            Save List
          </button>
        </div>
      </div>
    </div>
  );
}

export default PizzazListApp;
